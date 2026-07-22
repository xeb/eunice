# eunice.xeb.ai Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serve `https://eunice.xeb.ai` as an always-on `eunice --webapp` backed by the local `gemmad` daemon, gated by Cloudflare Access to `xebxeb@gmail.com`, plus a scheduled agent that emails a daily system-health report.

**Architecture:** A systemd user service runs `eunice --webapp --gemmad` bound to `127.0.0.1:8812`. The existing `cloudflared` tunnel routes `eunice.xeb.ai` to that loopback origin; a Cloudflare Access self-hosted app is the only authentication layer in front of the webapp's full shell. A `daily-health-email` scheduled agent (07:00) collects host facts and sends them via `gmail-cli`.

**Tech Stack:** Rust (eunice, already built), systemd user services, cloudflared tunnel, Cloudflare Access + DNS APIs, gemmad (llama.cpp), gmail-cli.

## Global Constraints

- Origin MUST bind `127.0.0.1:8812` — never `0.0.0.0`. The webapp exposes an un-authenticated shell; loopback bind + Access are both load-bearing.
- Cloudflare Access app MUST exist BEFORE the DNS record is published (no briefly-public shell).
- This plan document and the eunice repo MUST NOT contain any secret values. Cloudflare tokens are read from `~/p/master/docs/cloudflare.md` into shell variables at runtime; the gemmad Bearer token is resolved by eunice from `~/.config/gemmad/keys.toml` (never copied).
- Cloudflare identifiers (verbatim): account `6af766c6daa737e50eff404ae8a579d8`, zone `xeb.ai` = `91b65806dd3c138bf2f417778bc01431`, tunnel `f9a2e2e8-5e9e-4fa3-a99f-4a201a419e26`, team domain `kockerbeck.cloudflareaccess.com`.
- Access policy: allow `xebxeb@gmail.com` only (Mark-only).
- Cron dialect: `agents.toml` uses standard 5-field Unix cron.
- eunice binary in use: `~/.cargo/bin/eunice` (already installed).

---

### Task 1: Working directory + daily-health-email agents.toml

**Files:**
- Create: `/home/xeb/.eunice/webapp/` (directory)
- Create: `/home/xeb/.eunice/webapp/agents.toml`

**Interfaces:**
- Produces: a valid `agents.toml` at `/home/xeb/.eunice/webapp/agents.toml` with one agent `daily-health-email`, consumed by Task 2's `--install --agents`.

- [ ] **Step 1: Create the working directory**

Run: `mkdir -p /home/xeb/.eunice/webapp`
Expected: no output; `test -d /home/xeb/.eunice/webapp && echo OK` prints `OK`.

- [ ] **Step 2: Write the agents.toml**

Write `/home/xeb/.eunice/webapp/agents.toml` with exactly this content. `model` is intentionally omitted so the agent inherits the server's gemmad default. The prompt is explicit and deterministic so a Gemma-class model executes it reliably.

```toml
# Scheduled agents for the eunice.xeb.ai webapp.
# Standard 5-field Unix cron. Editable live in the web UI (Agents tab).

[[agent]]
name = "daily-health-email"
schedule = "0 7 * * *"
enabled = true
timeout_secs = 300
prompt = """
You are a system-health reporter running on Mark's Linux workstation. Collect the
facts below by running each shell command with the Bash tool, then email a concise
plain-text summary. Do NOT modify any service or file except the temp file you write.

Run these commands and capture their output:
1. hostname && uptime
2. df -h / /media/xeb/GreyArea
3. free -h
4. systemctl --user is-active gemmad cloudflared eunice authd
5. systemctl --user --failed --no-legend
6. curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18082/v1/models
   (gemmad requires auth, so 200 OR 400 both mean it is UP and answering; an empty
   response or 000 means it is DOWN — do not treat 400 as a failure.)
7. nvidia-smi --query-gpu=memory.used,memory.total,temperature.gpu --format=csv,noheader ; echo "(skip if nvidia-smi missing)"

Then compose the report:
- First line: an overall verdict — "OK" if every service in step 4 is "active", there
  are no failed units in step 5, the step-6 HTTP code is 200 or 400 (gemmad up), and
  disk use on both filesystems is under 90%. Otherwise "ATTENTION" followed by the
  specific problems.
- Below the verdict, a short labeled section for each of: Host (hostname + uptime),
  Disk, Memory, Services, Failed units, gemmad (the HTTP code, 200 = healthy), GPU.
- Keep it scannable. Plain text, no markdown tables.

Write the full report to a temp file, e.g. /tmp/eunice-health.txt (use the Write tool),
then send it by running exactly:
  gmail-cli compose --to xebxeb@gmail.com --subject "eunice health — <HOSTNAME> <YYYY-MM-DD>" --body-file /tmp/eunice-health.txt
Substitute the real hostname and today's date into the subject. If gmail-cli exits
non-zero, report the error text in your final message. Your final message should state
whether the email was sent and the one-line verdict.
"""
```

- [ ] **Step 3: Verify the TOML parses**

Run: `python3 -c "import tomllib; d=tomllib.load(open('/home/xeb/.eunice/webapp/agents.toml','rb')); a=d['agent'][0]; print(a['name'], a['schedule'], a['enabled'], a['timeout_secs'])"`
Expected: `daily-health-email 0 7 * * * True 300`

- [ ] **Step 4: Verify gmail-cli is available and authed (prerequisite for the agent)**

Run: `gmail-cli status`
Expected: contains `Logged in as: xebxeb@gmail.com`. If not, stop and run `gmail-cli login` in a tmux pane first.

_No commit — this file lives outside the repo (machine-specific runtime config)._

---

### Task 2: Install and configure the eunice.service systemd unit

**Files:**
- Create (via `eunice --install`, then rewrite): `/home/xeb/.config/systemd/user/eunice.service`
- Modify: `/home/xeb/.eunice/eunice.env` (append gemmad connection vars)

**Interfaces:**
- Consumes: `/home/xeb/.eunice/webapp/agents.toml` (Task 1).
- Produces: a running webapp on `127.0.0.1:8812` backed by gemmad; consumed by Tasks 4–6.

- [ ] **Step 0: Ensure the installed binary matches the repo (do NOT assume it does)**

The systemd unit runs `~/.cargo/bin/eunice`, which may be a stale `cargo install` older
than the repo — serving an outdated webapp UI. Check and refresh:
```bash
/home/xeb/.cargo/bin/eunice --version    # installed
( cd /media/xeb/GreyArea/projects/eunice && grep '^version' Cargo.toml )   # repo
```
If they differ, rebuild+install before continuing (release build ~2–3 min):
```bash
cd /media/xeb/GreyArea/projects/eunice && cargo install --path . --force
/home/xeb/.cargo/bin/eunice --version    # must now match the repo
```
(Observed during execution: installed was `1.0.6`, repo `1.0.8` — the pre-full-page agent
editor UI. Reinstalling fixed it; if the service is already running, restart it after.)

- [ ] **Step 1: Confirm gemmad is up (needed so the webapp resolves its model at startup)**

Run: `systemctl --user is-active gemmad && curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:18082/v1/models`
Expected: `active` then `400` (gemmad rejects the unauthenticated probe with 400 — that response confirms it is up; `000`/connection-refused would mean down). Authenticated requests from eunice (which resolves the `dev` token) get `200`.

- [ ] **Step 2: Run the installer (bootstraps unit + env snapshot + enable + linger)**

Run:
```bash
eunice --webapp --host 127.0.0.1 --port 8812 --agents /home/xeb/.eunice/webapp/agents.toml --install
```
Expected: prints `Validated agents file: …`, `Wrote unit file: …/eunice.service`, `enabled`, `Started`/`Restarted …`, `Install complete!`. (The snapshot captures PATH incl. `~/.local/bin` + `~/.cargo/bin` — verified present — so the agent's Bash tool will find `gmail-cli`.)

- [ ] **Step 3: gemmad connection env — NOT NEEDED (verified during execution)**

`GEMMAD_HOST`/`GEMMAD_PORT` already default to `127.0.0.1`/`18082`, and `resolve_token()`
falls back to the default keys path `~/.config/gemmad/keys.toml` (the `dev` key) when
`GEMMAD_KEYS_FILE` is unset. Confirmed at runtime: the service log printed
`Using local gemmad (gemma-4-26b-a4b) at 127.0.0.1:18082` with no env set — retrieving the
real model id requires a successful authenticated call, so the token resolves. **Skip this
step.** (Adding the vars would be redundant; omitted per YAGNI.)

- [ ] **Step 4: Rewrite the unit with --gemmad, gemmad ordering, and the stable WorkingDirectory**

`eunice --install` omits `--gemmad` and any gemmad ordering, and set WorkingDirectory to the install cwd. Overwrite the unit file with exactly this content:

```ini
[Unit]
Description=Eunice agentic webapp server
After=network-online.target gemmad.service
Wants=gemmad.service

[Service]
Type=simple
ExecStart=/home/xeb/.cargo/bin/eunice --webapp --port 8812 --host 127.0.0.1 --gemmad --agents /home/xeb/.eunice/webapp/agents.toml
WorkingDirectory=/home/xeb/.eunice/webapp
ExecReload=/bin/kill -HUP $MAINPID
EnvironmentFile=-/home/xeb/.eunice/eunice.env
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

(Write this to `/home/xeb/.config/systemd/user/eunice.service`. Confirm the eunice path first with `command -v eunice` — it should be `/home/xeb/.cargo/bin/eunice`; if different, use that absolute path in ExecStart.)

- [ ] **Step 5: Reload systemd and restart the service**

Run:
```bash
systemctl --user daemon-reload
systemctl --user restart eunice.service
sleep 3
systemctl --user is-active eunice.service
```
Expected: `active`. If `activating`/`failed`, check `journalctl --user -u eunice -n 40 --no-pager` — a fast exit usually means gemmad wasn't reachable (retry loop) or a config error.

- [ ] **Step 6: Verify the loopback bind (critical security check)**

Run: `ss -tlnp | grep 8812`
Expected: a line showing `127.0.0.1:8812` and process `eunice`. There MUST be NO `0.0.0.0:8812`. If it shows `0.0.0.0`, stop and fix the `--host` before going further.

- [ ] **Step 7: Verify the app serves and gemmad is wired correctly**

Run:
```bash
curl -s -o /dev/null -w 'app:%{http_code}\n' http://127.0.0.1:8812/
journalctl --user -u eunice --since '2 min ago' --no-pager | grep -iE 'gemmad|model|error|token' | tail -20
```
Expected: `app:200`, and the log shows eunice selected gemmad (e.g. the gemmad model id / `18082`) with NO `gemmad token not found`, connection-refused, or panic lines. (A definitive live Gemma reply is confirmed end-to-end in Task 6 Step 1's browser test — the webapp chat is SSE/session-based and awkward to drive from curl, so it is not scripted here.) If the log shows a cloud model was chosen instead of gemmad, the `--gemmad` flag or `GEMMAD_*` env is missing — recheck Steps 3–4.

_No repo commit — all artifacts are outside the repo._

---

### Task 3: Cloudflare Access application (Mark-only) — created BEFORE DNS

**Files:** none in repo. Cloudflare API only.

**Interfaces:**
- Produces: `APP_ID` for `eunice.xeb.ai`, recorded for the Task 6 docs update.

> Each block below is self-contained (re-derives the token inline) because shell env vars do NOT persist across separate command invocations. The token value is never printed.

- [ ] **Step 1: Sanity-check that the Access token is derivable**

Run:
```bash
CF_ACCESS_TOKEN=$(grep 'Claude v3' /home/xeb/p/master/docs/cloudflare.md | grep -oE 'cfut_[A-Za-z0-9]+' | head -1)
test -n "$CF_ACCESS_TOKEN" && echo "token loaded (${#CF_ACCESS_TOKEN} chars)" || echo "TOKEN NOT FOUND"
```
Expected: `token loaded (N chars)` with N a positive number (length only — never print the value). If `TOKEN NOT FOUND`, the `cloudflare.md` row label changed — locate the "Apps and Policies" token row and adjust the grep.

- [ ] **Step 2: Create the self-hosted Access app AND attach the Mark-only policy (one block)**

Run:
```bash
CF_ACCOUNT=6af766c6daa737e50eff404ae8a579d8
CF_ACCESS_TOKEN=$(grep 'Claude v3' /home/xeb/p/master/docs/cloudflare.md | grep -oE 'cfut_[A-Za-z0-9]+' | head -1)
APP_ID=$(curl -s -X POST "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT/access/apps" \
  -H "Authorization: Bearer $CF_ACCESS_TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"Eunice","domain":"eunice.xeb.ai","type":"self_hosted","session_duration":"24h"}' \
  | python3 -c "import sys,json; r=json.load(sys.stdin); assert r['success'], r['errors']; print(r['result']['id'])")
echo "app_id=$APP_ID"
curl -s -X POST "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT/access/apps/$APP_ID/policies" \
  -H "Authorization: Bearer $CF_ACCESS_TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"Mark only","decision":"allow","include":[{"email":{"email":"xebxeb@gmail.com"}}],"session_duration":"24h","precedence":1}' \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print('policy_success', r['success'], r['result']['name'] if r['success'] else r['errors'])"
```
Expected: `app_id=<UUID>` then `policy_success True Mark only`.

- [ ] **Step 3: Verify the app is registered for the hostname (self-contained)**

Run:
```bash
CF_ACCOUNT=6af766c6daa737e50eff404ae8a579d8
CF_ACCESS_TOKEN=$(grep 'Claude v3' /home/xeb/p/master/docs/cloudflare.md | grep -oE 'cfut_[A-Za-z0-9]+' | head -1)
curl -s "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT/access/apps" \
  -H "Authorization: Bearer $CF_ACCESS_TOKEN" \
  | python3 -c "import sys,json; [print(a['id'],a['domain']) for a in json.load(sys.stdin)['result'] if a['domain']=='eunice.xeb.ai']"
```
Expected: exactly one line: `<APP_ID> eunice.xeb.ai`. (If more than one, delete the duplicate app before continuing.)

---

### Task 4: Add the cloudflared tunnel ingress

**Files:**
- Modify: `/home/xeb/.cloudflared/config.yml` (+ timestamped backup)

**Interfaces:**
- Consumes: the running origin from Task 2 (`127.0.0.1:8812`).
- Produces: tunnel routing for `eunice.xeb.ai` (inert until Task 5 publishes DNS).

- [ ] **Step 1: Back up the current tunnel config**

Run: `cp /home/xeb/.cloudflared/config.yml "/home/xeb/.cloudflared/config.yml.bak.$(date +%Y%m%d-%H%M%S)"`
Expected: no output; `ls /home/xeb/.cloudflared/config.yml.bak.* | tail -1` shows a fresh backup.

- [ ] **Step 2: Insert the eunice ingress above the 404 catch-all**

Edit `/home/xeb/.cloudflared/config.yml`: immediately before the final `- service: http_status:404` line, insert:
```yaml
  - hostname: eunice.xeb.ai
    service: http://localhost:8812
```
Verify placement: `grep -n -A1 'eunice.xeb.ai' /home/xeb/.cloudflared/config.yml` shows the hostname followed by the `service: http://localhost:8812` line, and `grep -n 'http_status:404' /home/xeb/.cloudflared/config.yml` is on a LATER line number.

- [ ] **Step 3: Validate and restart cloudflared**

Run:
```bash
cloudflared tunnel ingress validate --config /home/xeb/.cloudflared/config.yml
systemctl --user restart cloudflared.service
sleep 3
systemctl --user is-active cloudflared.service
```
Expected: ingress validation prints `OK` (or "Validating rules… OK"), then `active`. If validate fails, restore the backup and fix the YAML before restarting.

---

### Task 5: Publish the DNS record (makes eunice.xeb.ai live)

**Files:** none in repo. Cloudflare API only.

**Interfaces:**
- Consumes: the Access app (Task 3) — MUST already exist. The tunnel ingress (Task 4).

- [ ] **Step 1: Confirm the Access app exists first (guard against a public shell)**

Run (self-contained):
```bash
CF_ACCOUNT=6af766c6daa737e50eff404ae8a579d8
CF_ACCESS_TOKEN=$(grep 'Claude v3' /home/xeb/p/master/docs/cloudflare.md | grep -oE 'cfut_[A-Za-z0-9]+' | head -1)
curl -s "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT/access/apps" \
  -H "Authorization: Bearer $CF_ACCESS_TOKEN" \
  | python3 -c "import sys,json; m=[a for a in json.load(sys.stdin)['result'] if a['domain']=='eunice.xeb.ai']; print('GATE_READY' if m else 'MISSING'); [print(a['id']) for a in m]"
```
Expected: `GATE_READY` and the app id. **Do NOT proceed if it prints `MISSING`** — publishing DNS without the gate would expose a public shell.

- [ ] **Step 2: Create the proxied CNAME → tunnel (self-contained)**

Run:
```bash
CF_ZONE=91b65806dd3c138bf2f417778bc01431
CF_DNS_TOKEN=$(grep 'DNS Management' /home/xeb/p/master/docs/cloudflare.md | grep -oE 'cf[ua]t_[A-Za-z0-9]+' | head -1)
curl -s -X POST "https://api.cloudflare.com/client/v4/zones/$CF_ZONE/dns_records" \
  -H "Authorization: Bearer $CF_DNS_TOKEN" -H "Content-Type: application/json" \
  -d '{"type":"CNAME","name":"eunice","content":"f9a2e2e8-5e9e-4fa3-a99f-4a201a419e26.cfargotunnel.com","proxied":true,"ttl":1}' \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print('success',r['success']); print(r['result'].get('name'), r['result'].get('proxied')) if r['success'] else print(r['errors'])"
```
Expected: `success True` and `eunice.xeb.ai True`. (If it reports the record already exists, GET records filtered to `name=eunice.xeb.ai`, then PATCH that record id to `type=CNAME, content=f9a2e2e8-…​.cfargotunnel.com, proxied=true`.)

- [ ] **Step 3: Verify the gate is live at the edge**

Run (allow ~30–60s for propagation; retry if needed):
```bash
curl -sI https://eunice.xeb.ai | head -n 5
```
Expected: `HTTP/2 302` with a `location:` header pointing at `https://kockerbeck.cloudflareaccess.com/...`. That 302 proves the Access gate is intercepting — the shell is NOT publicly reachable. A `200` here would mean the gate is missing — treat as a security failure and re-check Task 3.

---

### Task 6: End-to-end verification, agent test, and docs update

**Files:**
- Modify: `/home/xeb/p/master/docs/cloudflare.md` (in the master repo — commit there)
- Modify: this eunice repo — commit the spec + plan (already added) on branch `eunice-xeb-ai-deploy`

- [ ] **Step 1: Browser end-to-end (human-in-the-loop)**

Ask Mark to open `https://eunice.xeb.ai`, complete the Cloudflare Access Google login as `xebxeb@gmail.com`, and confirm the eunice webapp loads and a chat reply comes back from gemmad.
Expected: login succeeds only for `xebxeb@gmail.com`; the app loads; a message gets a Gemma reply.

- [ ] **Step 2: Trigger the health agent once (no run-now API → temporary cron 2 min ahead)**

Set the schedule to ~2 minutes in the future (local time; the scheduler uses `chrono::Local`), reload via SIGHUP, and record the target time:
```bash
python3 - <<'PY'
import re, subprocess
mmhh = subprocess.check_output(["date","-d","+2 min","+%-M %-H"]).decode().split()
mm, hh = mmhh[0], mmhh[1]
p = "/home/xeb/.eunice/webapp/agents.toml"
s = open(p).read()
s = re.sub(r'schedule = "[^"]*"', f'schedule = "{mm} {hh} * * *"', s, count=1)
open(p, "w").write(s)
print(f"temporary schedule set to '{mm} {hh} * * *' — fires at {hh}:{int(mm):02d} local")
PY
systemctl --user reload eunice.service
```
Wait until just past the target minute, then inspect the agent's run state (print the full JSON — field names vary, read whatever status/timestamp it exposes):
```bash
curl -s http://127.0.0.1:8812/api/agents \
  | python3 -c "import sys,json; d=json.load(sys.stdin); a=[x for x in (d if isinstance(d,list) else d.get('agents',[])) if x.get('name')=='daily-health-email']; print(json.dumps(a[0], indent=2) if a else 'agent not found')"
```
Expected: the JSON shows a recent run with a success/ok status and a run timestamp within the last couple of minutes. **Primary pass signal:** confirm with Mark that an email titled `eunice health — …` arrived at `xebxeb@gmail.com` with real host facts. Also skim `journalctl --user -u eunice --since '3 min ago' --no-pager` for the run and any `gmail-cli`/model errors.

- [ ] **Step 3: Restore the real schedule**

Run:
```bash
python3 - <<'PY'
import re
p = "/home/xeb/.eunice/webapp/agents.toml"
s = open(p).read()
s = re.sub(r'schedule = "[^"]*"', 'schedule = "0 7 * * *"', s, count=1)
open(p, "w").write(s)
print("restored schedule to '0 7 * * *'")
PY
systemctl --user reload eunice.service
grep -n 'schedule' /home/xeb/.eunice/webapp/agents.toml
```
Expected: `schedule = "0 7 * * *"`. (If the Step 2 run failed by using a cloud model instead of gemmad — e.g. the log shows a non-gemmad model or an `ANTHROPIC`/`OPENAI` call — add `model = "gemma-4-26b-a4b"` to the agent, reload, and re-test before restoring.)

- [ ] **Step 4: Reboot-survival check**

Run: `systemctl --user is-enabled eunice.service && loginctl show-user "$USER" -p Linger`
Expected: `enabled` and `Linger=yes` (so it survives logout/reboot). Then `systemctl --user restart eunice.service && sleep 5 && systemctl --user is-active eunice.service` → `active` (reconnects to gemmad).

- [ ] **Step 5: Update cloudflare.md**

First re-fetch the app id (state does not persist from Task 3):
```bash
CF_ACCOUNT=6af766c6daa737e50eff404ae8a579d8
CF_ACCESS_TOKEN=$(grep 'Claude v3' /home/xeb/p/master/docs/cloudflare.md | grep -oE 'cfut_[A-Za-z0-9]+' | head -1)
curl -s "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT/access/apps" \
  -H "Authorization: Bearer $CF_ACCESS_TOKEN" \
  | python3 -c "import sys,json; [print(a['id']) for a in json.load(sys.stdin)['result'] if a['domain']=='eunice.xeb.ai']"
```

Then, in `/home/xeb/p/master/docs/cloudflare.md`:
- Add to the **"Tunnel-hosted sites (not Pages)"** table: `| eunice.xeb.ai | localhost:8812 via tmux-proxy tunnel | Access-gated, Mark-only — eunice --webapp + gemmad, systemd user service eunice.service. **Full shell — never expose without the Access gate.** |`
- Add to the **"Active Access Applications"** table: `| Eunice | eunice.xeb.ai | Mark only (xebxeb@gmail.com) | <the app id printed above> |`

Commit in the master repo:
```bash
git -C /home/xeb/p/master add docs/cloudflare.md
git -C /home/xeb/p/master commit -m "docs: add eunice.xeb.ai (tunnel-hosted, Access-gated) Access app + tunnel site"
```

- [ ] **Step 6: Commit the plan doc in the eunice repo**

Run:
```bash
git -C /media/xeb/GreyArea/projects/eunice add docs/superpowers/plans/2026-07-22-eunice-xeb-ai-deploy.md
git -C /media/xeb/GreyArea/projects/eunice commit -m "Add implementation plan for eunice.xeb.ai deployment"
```

---

## Final verification checklist (all must pass)

- [ ] `curl -s http://127.0.0.1:8812/api/status` reports the current repo `version` (not a stale one) → served UI is up to date.
- [ ] `ss -tlnp | grep 8812` → `127.0.0.1:8812` only (no `0.0.0.0`).
- [ ] `curl -sI https://eunice.xeb.ai` → `302` to `kockerbeck.cloudflareaccess.com`.
- [ ] Browser login as `xebxeb@gmail.com` → app loads, gemma reply.
- [ ] `systemctl --user is-enabled eunice.service` → `enabled`; `Linger=yes`.
- [ ] Health email received at `xebxeb@gmail.com`; agent schedule restored to `0 7 * * *`.
- [ ] `cloudflare.md` updated (tunnel-sites + Access-apps tables) and committed.
