# How to Build Scheduled Long-Running Agents

A scheduled agent in eunice is a prompt plus a cron schedule. The webapp server keeps a scheduler
running in-process; when an agent is due, the server starts a normal session, runs the agent loop
with the full Bash/Read/Write/Skill toolset, and saves the transcript where you can read it in the
browser.

This guide takes you from a single working agent to an always-on systemd service.

---

## 1. Quickstart

Create a file called `agents.toml`:

```toml
[[agent]]
name = "hello-agent"
schedule = "*/5 * * * *"
prompt = "Write the current date and time to /tmp/eunice-hello.txt, then say what you wrote."
```

Start the webapp with it:

```bash
eunice --webapp --agents agents.toml
```

### What you see in the terminal

The server validates `agents.toml` before it binds the port, then the scheduler prints the agents it
is watching:

```
Session persistence: sessions.db
Tools available: 4
Scheduled agents: 1 (from /home/me/agents.toml)
[14:22:07.104] [scheduler] watching 1 enabled agent(s)
[14:22:07.104] [scheduler]   hello-agent — "*/5 * * * *" (as "0 */5 * * * *")
Starting webapp server at http://0.0.0.0:8811
Press Ctrl+C to stop
```

Within five minutes the first run fires:

```
[14:25:00.012] [scheduler] [hello-agent] run starting in session 3f9a2c1b (chrome-molly)
[14:25:06.771] [scheduler] [hello-agent] run complete
```

### What you see in the browser

Open <http://localhost:8811>, click the hamburger menu, and the drawer now has two tabs:
**SESSIONS** and **AGENTS**. (The AGENTS tab only appears when the server was started with
`--agents`.)

The AGENTS tab shows one card per agent with a status dot, the cron expression, the model, and when
the next and last runs are. Click a card to expand it: you get the prompt preview, `working_dir`,
timeout, the last error if there was one, and the ten most recent runs. Click a run to open its full
transcript. The list refreshes every 30 seconds while the AGENTS tab is the one you are looking at.

Over in the SESSIONS tab, sessions produced by an agent are marked with a ⏰ badge.

---

## 2. Every field of an `[[agent]]` table

An `agents.toml` is a list of `[[agent]]` tables. Unknown keys are rejected outright, so a typo like
`timeout = 900` is a startup error rather than a silently ignored setting.

| Field | Required | Default | Rule |
| --- | --- | --- | --- |
| `name` | yes | — | Lowercase kebab-case, unique across the file |
| `schedule` | yes | — | Standard 5-field Unix cron |
| `prompt` | one of the two | — | Inline prompt text, non-empty |
| `prompt_file` | one of the two | — | Path to a prompt file, read at startup |
| `model` | no | the server's model | Must resolve to a known provider |
| `enabled` | no | `true` | Boolean |
| `timeout_secs` | no | `600` | Integer greater than 0 |
| `working_dir` | no | the server's cwd | Must exist and be a directory |

### `name`

Required and unique. It must be 1–64 characters, start with a lowercase letter or a digit, and
contain only lowercase letters, digits, and hyphens. `daily-digest` and `repo-watch2` are valid;
`Daily`, `-lead`, `has space`, and `under_score` are not.

The name is what ties a session back to its agent — it is stored on the session row, drives the ⏰
badge, and is how the AGENTS tab finds an agent's run history.

```toml
name = "morning-digest"
```

### `schedule`

Required. A standard 5-field Unix cron expression, evaluated in the server's local timezone. See
[section 3](#3-writing-schedules) for the details.

```toml
schedule = "0 7 * * 1-5"
```

### `prompt` and `prompt_file`

Set **exactly one** of them. Setting both, or neither, is a startup error.

`prompt` is the prompt text inline. TOML multi-line strings work well for anything longer than a
sentence:

```toml
prompt = """
Check the disk usage of /var and /home.
If either is above 85% full, write a warning line to ~/alerts.log with the timestamp.
Otherwise write nothing.
"""
```

`prompt_file` points at a file whose contents become the prompt. **Relative paths resolve against the
directory containing `agents.toml`**, not against the server's working directory — so a config at
`/home/me/agents/agents.toml` with `prompt_file = "prompts/digest.md"` reads
`/home/me/agents/prompts/digest.md`. Absolute paths are used as given.

```toml
prompt_file = "prompts/morning-digest.md"
```

The file is read **once at startup**, not on each run. Editing the prompt file requires a server
restart, exactly like editing `agents.toml` itself.

Either way, the resolved prompt must not be blank; a prompt that is only whitespace fails validation.

### `model`

Optional. Defaults to whatever model the server itself is running (`--model`, or the smart default).
Any model string eunice accepts on the command line works here, including aliases:

```toml
model = "flash"
```

The model is resolved at startup, so a typo fails immediately rather than at 3 a.m. If an agent's
model differs from the server's, the server builds a dedicated HTTP client for it at startup — which
means the credentials for that provider have to be present in the server's environment.

### `enabled`

Optional, default `true`. A disabled agent is still parsed, still validated, and still shown in the
AGENTS tab (greyed out with a `DISABLED` label), but it is never scheduled and has no next run time.
This is the clean way to park an agent without deleting it.

```toml
enabled = false
```

### `timeout_secs`

Optional, default `600` (ten minutes). Must be greater than zero. See
[section 5](#5-scheduling-semantics-worth-knowing) for exactly what a timeout does.

```toml
timeout_secs = 1800
```

### `working_dir`

Optional. When set, this agent gets its own tool registry whose Bash commands run with this
directory as their cwd, and whose Read/Write tools resolve relative paths against it. Agents without
a `working_dir` share the server's registry and run in the server's working directory.

The directory must exist when the server starts, and the path is canonicalized (symlinks resolved)
at load time.

```toml
working_dir = "/home/me/p/myrepo"
```

### A fully populated example

```toml
[[agent]]
name = "repo-watch"
schedule = "*/30 9-17 * * MON-FRI"
prompt_file = "prompts/repo-watch.md"
model = "flash"
enabled = true
timeout_secs = 900
working_dir = "/home/me/p/myrepo"
```

---

## 3. Writing schedules

`agents.toml` takes **standard 5-field Unix cron**:

```
┌───────────── minute        (0-59)
│ ┌─────────── hour          (0-23)
│ │ ┌───────── day of month  (1-31)
│ │ │ ┌─────── month         (1-12 or JAN-DEC)
│ │ │ │ ┌───── day of week   (0-7 or SUN-SAT)
│ │ │ │ │
* * * * *
```

Runs fire at second zero of the matching minute.

### Common schedules

| Expression | Fires |
| --- | --- |
| `*/5 * * * *` | Every 5 minutes |
| `*/30 * * * *` | Every 30 minutes |
| `0 * * * *` | Every hour, on the hour |
| `0 */4 * * *` | Every 4 hours |
| `0 9 * * *` | Every day at 09:00 |
| `30 6 * * *` | Every day at 06:30 |
| `0 9,17 * * *` | Every day at 09:00 and 17:00 |
| `0 7 * * 1-5` | Weekdays at 07:00 |
| `0 7 * * MON-FRI` | Weekdays at 07:00 (same thing, by name) |
| `*/15 9-17 * * MON-FRI` | Every 15 minutes, 09:00–17:59, weekdays |
| `0 9 * * 1` | Mondays at 09:00 |
| `0 22 * * 0` | Sundays at 22:00 |
| `0 22 * * 7` | Sundays at 22:00 (7 is also Sunday) |
| `0 3 1 * *` | The 1st of every month at 03:00 |
| `0 3 1 1,4,7,10 *` | Quarterly, on the 1st at 03:00 |

### Day-of-week

The day-of-week field accepts `0` through `7`, where **both `0` and `7` mean Sunday**. Names work
too — `SUN`, `MON`, … `SAT` — and so do ranges (`MON-FRI`), lists (`1,3,5`), and steps (`1-5/2`).

### Why eunice logs a second, different-looking expression

At startup the scheduler prints each agent's schedule twice:

```
[scheduler]   weekday-report — "0 7 * * 1-5" (as "0 0 7 * * 2-6")
```

The second form is internal. The cron library eunice uses is not standard-cron compatible: it wants a
leading **seconds** field (six fields, not five) and it numbers day-of-week `1=Sunday…7=Saturday`,
where Unix cron uses `0=Sunday…6=Saturday`. Handed `0 9 * * 1` directly, that library would fire on
**Sunday**, not Monday.

So eunice translates for you. `normalize_cron` prepends a `0` seconds field and shifts bare
day-of-week integers by one — inside lists, ranges, and steps alike — while leaving `*` and
alphabetic names untouched. That is why `1-5` shows up as `2-6` in the log line and why `MON-FRI`
shows up unchanged.

**You always write standard 5-field cron.** Never write the 6-field form in `agents.toml`; a
6-field expression is rejected with `got 6 fields`. The translated form is printed purely so that if
an agent fires on a day you did not expect, you can see exactly what the scheduler was given.

### One real difference from Unix cron

eunice translates the day-of-week *numbering* only. When an expression restricts **both**
day-of-month and day-of-week, the two behave differently:

| Expression | Classic Unix cron | eunice |
|---|---|---|
| `0 0 1 * 1` | the 1st **or** any Monday | only a 1st that **is** a Monday |

Unix cron takes the union; eunice takes the intersection. So `0 0 1 * 1` fires roughly once every
few months rather than several times a month.

If you meant "or", write two agents:

```toml
[[agent]]
name = "monthly-report"
schedule = "0 0 1 * *"          # the 1st, whatever day it lands on
prompt_file = "prompts/report.md"

[[agent]]
name = "weekly-report"
schedule = "0 0 * * MON"        # every Monday
prompt_file = "prompts/report.md"
```

The scheduler prints a `WARNING` at startup naming any agent whose schedule restricts both day
fields, so you will not hit this silently. Restricting just one of them — which is what almost every
real schedule does — is unaffected.

---

## 4. What happens on a run

When an agent comes due:

1. **A session is created and tagged with the agent name.** It gets an ordinary generated session
   name (`chrome-molly`, `neon-wintermute`, and so on) — the agent name lives in a separate column,
   which is what the ⏰ badge and the run history in the AGENTS tab are built from.
2. **The webapp's agent loop runs the prompt**, exactly the same loop an interactive browser query
   uses, with the same four tools: Bash, Read, Write, and Skill. If the server was started with a
   system prompt (`--prompt`, or an auto-discovered `prompt.md`), it is prepended to the agent's
   prompt on this first turn, just as it is for a new interactive session.
3. **The transcript is persisted as it is produced** — user turn, assistant messages, tool calls, and
   tool output all land in the session, so a finished run reads like any other conversation.
4. **You can watch it live.** The run publishes a broadcast channel and marks its session as running,
   so opening the run from the AGENTS tab while it is in flight attaches the browser to the live
   event stream. Clicking a run under RECENT RUNS restores the history so far and then follows along.
5. **The outcome is recorded**: `success`, or `failed` with the error message. A failed run also gets
   a final assistant message appended to its own transcript, reading
   `Agent '<name>' run failed: <message>`, so the stored history never just stops mid-turn with no
   explanation.

Runs are independent of interactive traffic. They do not interfere with a query you are typing in the
browser, and the browser's cancel button does not stop them.

---

## 5. Scheduling semantics worth knowing

**Missed schedules are not backfilled.** The scheduler only fires occurrences that fall in the window
since its last tick while it was running. If the server was down at 09:00, the 09:00 run does not
happen — not when the server comes back up, not ever. It is skipped, not replayed. This is the main
reason to run eunice as a service rather than in a terminal you might close.

**An overrunning agent skips its own next tick.** There is at most one in-flight run per agent. If a
run on `*/5 * * * *` takes seven minutes, the ticks that arrive while it is still going are recorded
as `skipped` and the schedule moves on — they do not queue up behind the slow run. The scheduler logs
each one:

```
[scheduler] [repo-watch] previous run still in flight, skipping this tick
```

If you see a lot of these, raise the interval or lower the work the prompt is doing.

**Different agents run concurrently.** The one-run-at-a-time rule is per agent, not global. Three
agents scheduled at `0 9 * * *` all start at 09:00 together.

**Times are the server's local timezone.** Not UTC — whatever the machine running the server thinks
local time is. Under systemd, that is the system timezone (`timedatectl`); a `TZ` set only in your
login shell is not carried into the service.

**`timeout_secs` bounds a run, and a timed-out run still leaves a transcript.** When a run exceeds
its timeout, eunice does not kill it outright — it signals the loop to stop and gives it 30 seconds
to unwind. That matters: the loop writes its transcript in one batch when it exits, so a run that is
dropped mid-flight loses its entire history. A run that stops cleanly is marked failed with
`timed out after <N>s` and keeps its partial transcript.

If the run does not stop within those 30 seconds — typically because it is parked in a long Bash
command or a slow API call, since cancellation is only checked between loop iterations — it is
abandoned. The status becomes `timed out after <N>s and did not stop cleanly`, and the session keeps
only the failure note. The log shows both stages:

```
[scheduler] [slow-agent] timed out after 600s, winding the run down
[scheduler] [slow-agent] did not wind down within 30s, abandoning the run
```

**Run state is in memory.** Last status, last error, and last run time are rebuilt from scratch when
the server restarts. The runs themselves are not lost — they are sessions in `sessions.db` — but the
AGENTS tab will show "never run" for an agent until it fires again after a restart.

---

## 6. Viewing agents in the web UI

The AGENTS tab lives in the hamburger drawer, next to SESSIONS. It appears only when the server was
started with `--agents`; without it, the drawer looks exactly as it always did.

Each card carries a status dot:

| Dot | Meaning |
| --- | --- |
| Cyan, pulsing | A run is in flight right now |
| Green | The last run succeeded |
| Red | The last run failed (or timed out) |
| Yellow | The last tick was skipped because the previous run was still going |
| Grey | Never run, or the agent is disabled |

The card summary shows the cron expression as you wrote it, the model (or `<server model> (default)`
when the agent does not set one), the next run as a relative time, and the last run. Expanding the
card adds the prompt preview (first 240 characters), `working_dir`, `timeout_secs`, the last error,
and up to ten recent runs.

### Editing agents from the browser

Expanding a card gives you an **EDIT** button and an enable/disable toggle, and the tab header has
**+ NEW AGENT**. Editing opens a modal with the agent's schedule, full prompt, model, timeout,
working directory and enabled flag. Saving writes `agents.toml` and applies immediately — no
restart.

A few things worth knowing about how saving behaves:

- **Your file is edited, not regenerated.** Comments, key order and formatting are preserved; only
  the keys you changed are touched. A hand-written, commented `agents.toml` survives a UI save
  intact.
- **`agents.toml.bak`** is written next to the config before each successful save.
- **Renaming is not supported.** Run history and past sessions are keyed to the agent's name, so the
  name field is read-only when editing. Create a new agent to use a different name.
- **Invalid input is rejected before anything is written**, with the same error messages the server
  prints at startup, shown inline in the modal. Your typing is kept.
- **If the file changed on disk** since the editor was opened — you edited it in a text editor, or
  another browser tab saved — the save is refused rather than clobbering it. The modal offers a
  reload and keeps your input.
- For an agent that uses `prompt_file`, the textarea edits **that file**, not the TOML. The modal
  shows which path it will write.
- Deleting an agent removes it from `agents.toml` but deliberately leaves any `prompt_file` on
  disk, since prompt files can be shared and are yours to manage.

> **Anyone who can reach the port can do this.** The webapp has no authentication of its own, so
> agent editing is available to whoever can load the page. See the `--host` warning in section 7 —
> bind to `127.0.0.1` unless you have an authenticating proxy in front.

### Changes apply without a restart

The server watches `agents.toml` and every file it references via `prompt_file`, and reloads within
a few seconds of a change — whether you edited it in the browser, in vim, or from a script.

If an edit is invalid, **the running config is kept** and the daemon stays up. The error appears in
the log and in the drawer, and the moment you fix the file it loads normally. That is the opposite
of startup behavior, where a bad config correctly refuses to start: once agents are running, a typo
must not take the service down.

You can also force a reload without touching the file:

```bash
systemctl --user reload eunice     # or: kill -HUP <pid>
```

---

## 7. Running it as a service

Scheduled agents only fire while the server is up, and missed occurrences are never replayed. For
anything you actually depend on, install the webapp as a **systemd user service** — no `sudo`, no
root.

### Before you install: two decisions that are baked in

**Which binary.** The unit's `ExecStart` is the *absolute path of the eunice you ran `--install`
with* (`std::env::current_exe()`). Install with the eunice on your `PATH`, not a build artifact:

```bash
which eunice          # /home/me/.cargo/bin/eunice  <- you want this one
```

If you run `--install` from `./target/release/eunice` inside a checkout, the service will point at
that build directory forever, and `cargo clean` will break it.

**Which directory.** `WorkingDirectory` is the directory you ran `--install` *from*, and that is
where `sessions.db` is created. Pick a stable home for it and `cd` there first:

```bash
mkdir -p ~/eunice-agents && cd ~/eunice-agents
```

### Installing

```bash
cd ~/eunice-agents
eunice --install --port 8811 --host 127.0.0.1 --agents ~/eunice-agents/agents.toml
```

```
Installing eunice as a systemd user service...

Validated agents file: /home/me/eunice-agents/agents.toml
Captured 6 API key(s) into /home/me/.eunice/eunice.env
Wrote unit file: /home/me/.config/systemd/user/eunice.service

Created symlink /home/me/.config/systemd/user/default.target.wants/eunice.service → /home/me/.config/systemd/user/eunice.service.
Started the service with the new configuration.

Install complete!
  Unit:   /home/me/.config/systemd/user/eunice.service
  URL:    http://127.0.0.1:8811
  Agents: /home/me/eunice-agents/agents.toml
  Logs:   journalctl --user -u eunice -f
```

The last line changes to `Restarted the service with the new configuration.` when a service is
already running, which is how you can tell a reinstall actually took effect.

`--install` accepts `--port`, `--host`, `--model`, `--agents`, `--prompt`, and `--no-persist`, and
bakes them into `ExecStart`. Note `--agents` requires `--webapp` or `--install`; using it alone is an
error. Prefer absolute paths for `--agents` — a relative one is resolved against `WorkingDirectory`,
which works, but is easy to get wrong later.

> **Bind to `127.0.0.1` unless you mean otherwise.** The default `--host` is `0.0.0.0`, which listens
> on every interface, and the webapp has **no authentication of its own** — it reads identity from
> proxy headers (`x-forwarded-email` and friends) and treats their absence as an anonymous user
> rather than rejecting the request. Since agents run with the Bash tool, anyone who can reach the
> port can run shell commands as you. On a laptop, a shared network, or a VPS, use
> `--host 127.0.0.1` and reach it over an SSH tunnel, or put it behind an authenticating reverse
> proxy. `--host 0.0.0.0` is only appropriate on a network you fully trust.

### What the installer does

1. **Validates `agents.toml` first.** Same validation the server performs. If the config is bad,
   nothing is written — the installer refuses to create a unit that would crash-loop.
2. **Snapshots your API keys** into `~/.eunice/eunice.env`, created with mode `0600` so the keys are
   never briefly world-readable. The variables captured, when set and non-empty, are:

   ```
   OPENAI_API_KEY   ANTHROPIC_API_KEY   GEMINI_API_KEY   GOOGLE_API_KEY   OLLAMA_HOST
   AZURE_OPENAI_ENDPOINT   AZURE_OPENAI_API_KEY   AZURE_OPENAI_API_VERSION
   GEMMAD_HOST   GEMMAD_PORT   GEMMAD_MODEL_ID   GEMMAD_API_KEY   GEMMAD_KEYS_FILE
   PATH
   ```

   This snapshot exists because **systemd user services do not inherit your login shell
   environment**. Whatever your `.bashrc` exports is invisible to the service. `PATH` is in the list
   for the same reason: systemd hands services a minimal `PATH`, and the Bash tool spawns commands
   with the service's environment — without the snapshot, an agent running `cargo` or `uv` or `gh`
   would get "command not found".

   If no keys are found, the installer prints a loud warning and continues; the service will start
   but every request will fail.
3. **Writes `~/.config/systemd/user/eunice.service`**, with `WorkingDirectory` set to the directory
   you ran `--install` from — that is where `sessions.db` will live — and `Restart=on-failure` with
   `RestartSec=5`.
4. **Runs `daemon-reload`, `enable`, and then `restart`.** The restart is unconditional and
   deliberate: `enable --now` is a no-op on an already-running unit, so a reinstall would otherwise
   keep serving the old configuration. Re-running `--install` is therefore the normal way to apply a
   config change.
5. **Enables lingering** (`loginctl enable-linger $USER`) so the service survives logout and starts
   at boot without a login session. Some distributions gate this behind polkit; if it fails you get a
   warning with the manual command rather than a failed install.

### The unit it writes

`~/.config/systemd/user/eunice.service`, for the install command above:

```ini
[Unit]
Description=Eunice agentic webapp server
After=network-online.target

[Service]
Type=simple
ExecStart=/home/me/.cargo/bin/eunice --webapp --port 8811 --host 127.0.0.1 --agents /home/me/eunice-agents/agents.toml
WorkingDirectory=/home/me/eunice-agents
EnvironmentFile=-/home/me/.eunice/eunice.env
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

The leading `-` on `EnvironmentFile` means "start anyway if the file is missing". `Restart=on-failure`
brings the server back if it crashes, but deliberately does **not** restart it after a clean exit.

### Checking it worked

```bash
systemctl --user status eunice
```

Look for `Active: active (running)`. Then confirm the scheduler actually picked up your agents —
this is the part worth checking, because a server with zero agents starts perfectly happily:

```bash
journalctl --user -u eunice --since "5 minutes ago" | grep scheduler
```

You want lines like:

```
[09:14:02.118] [scheduler] watching 2 enabled agent(s)
[09:14:02.118] [scheduler]   daily-digest — "0 9 * * *" (as "0 0 9 * * *")
[09:14:02.118] [scheduler]   repo-watch — "*/30 * * * *" (as "0 */30 * * * *")
```

`watching 0 enabled agent(s)` or `no enabled agents, scheduler idle` means every agent is
`enabled = false`, and no `[scheduler]` lines at all means the service was installed without
`--agents`. Either way, re-run `--install` with the right arguments.

You can also ask the running server directly:

```bash
curl -s http://127.0.0.1:8811/api/agents | python3 -m json.tool
```

`"agents_file": null` confirms it is running without a config.

### Changing the configuration

Agent definitions reload themselves; everything baked into `ExecStart` does not.

| Change | What to do |
|---|---|
| Edited `agents.toml` or a `prompt_file` | Nothing — it reloads within a few seconds |
| Edited an agent in the browser | Nothing — saving applies immediately |
| Changed port, host, model, or the agents path | Re-run `eunice --install` with the new flags |
| Rotated an API key | Re-run `eunice --install` (see below) |
| Upgraded eunice (`eunice --update`) | `systemctl --user restart eunice` |

Re-running `--install` overwrites the unit and restarts the service, so it is safe to repeat and is
the normal way to change anything baked into `ExecStart`.

That last row matters: `eunice --update` replaces the binary on disk, but a running service keeps
executing the old one until it is restarted. After updating, restart the service — otherwise
`eunice --version` and the daemon disagree.

### After a reboot

With lingering enabled the service starts at boot, before any login. Confirm with:

```bash
loginctl show-user "$USER" --property=Linger    # want Linger=yes
systemctl --user is-enabled eunice              # want enabled
```

If `Linger=no`, the installer's `enable-linger` call was refused; run
`loginctl enable-linger "$USER"` yourself (this one may prompt for authentication).

Missed schedules are not backfilled, so agents due during the downtime simply do not run — the next
occurrence after boot is the next one that fires.

### Managing it

```bash
systemctl --user status eunice        # is it up?
systemctl --user restart eunice       # apply an agents.toml change
systemctl --user stop eunice          # stop it
journalctl --user -u eunice -f        # follow the log, including scheduler output
journalctl --user -u eunice --since "1 hour ago"
```

### Rotating a key

The env file is a snapshot, not a link. After rotating an API key, export the new value in your shell
and **re-run `eunice --install`** — that rewrites `~/.eunice/eunice.env` and restarts the service.
(You can also edit that file by hand and `systemctl --user restart eunice`.)

### Removing it

```bash
eunice --uninstall-service
```

```
Removing the eunice systemd user service...

Removed /home/me/.config/systemd/user/default.target.wants/eunice.service.
Removed unit file: /home/me/.config/systemd/user/eunice.service

Uninstall complete!

Note: the following were left untouched:
  ~/.eunice/eunice.env  (snapshotted API keys)
  sessions.db           (webapp session history)
  user lingering        (disable with: loginctl disable-linger $USER)
```

It stops and disables the unit and deletes the unit file, and is safe to run when nothing is
installed. The three things it leaves behind are deliberate — your API keys and your session history
are not the installer's to throw away. Remove them yourself if you want a clean slate:

```bash
rm ~/.eunice/eunice.env          # only if you are done with the service entirely
loginctl disable-linger "$USER"  # only if nothing else of yours relies on lingering
```

`--install` and `--uninstall-service` require Linux with systemd, and cannot be combined with each
other or with `--uninstall`.

---

## 8. Troubleshooting

### The server refuses to start

Validation runs before the port is bound, so a bad config is a startup failure, not a silent
misbehavior. The error names the agent and the field. These are the real messages, from the config
loader (`src/agents.rs`) and from argument parsing (`src/main.rs`):

| Message | Cause |
| --- | --- |
| `failed to read agents file '<path>': ...` | The path passed to `--agents` does not exist or is unreadable |
| `failed to parse '<path>': ...` | Malformed TOML, or an **unknown key** in an `[[agent]]` table — unknown fields are rejected |
| `agent '<name>': name must be lowercase kebab-case (a-z, 0-9, hyphen) and start with a letter or digit` | Uppercase, spaces, underscores, a leading hyphen, empty, or longer than 64 characters |
| `duplicate agent name '<name>'` | Two `[[agent]]` tables share a name |
| ``agent '<name>': set exactly one of `prompt` or `prompt_file` `` | Both were set, or neither was |
| `agent '<name>': failed to read prompt_file '<path>': ...` | The prompt file is missing. The path shown is the resolved one — check it against the `agents.toml` directory |
| `agent '<name>': prompt is empty` | The prompt, or the prompt file's contents, is blank or whitespace only |
| `agent '<name>': timeout_secs must be greater than 0` | `timeout_secs = 0` |
| `agent '<name>': invalid schedule '<expr>': expected a 5-field cron expression (minute hour day-of-month month day-of-week), got N fields` | Wrong number of fields — 4 usually means a dropped `*`, 6 means you wrote the seconds-first form |
| `agent '<name>': invalid schedule '<expr>': day-of-week value N is out of range (expected 0-7)` | A day-of-week above 7 |
| `agent '<name>': invalid schedule '<expr>': cron expression '<expr>' is not valid: ...` | A field the cron parser rejects, such as a non-numeric minute |
| `agent '<name>': unknown model '<model>': ...` | The model does not resolve to a provider. Check spelling; for Ollama models, check the daemon is reachable |
| `agent '<name>': working_dir '<path>' does not exist` | Path is wrong, or is relative to somewhere you did not expect — `working_dir` is not resolved against the agents.toml directory |
| `agent '<name>': working_dir '<path>' is not a directory` | The path points at a file |
| `--agents requires --webapp or --install` | `--agents` was passed to a plain CLI invocation |

### Symptom table

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| The AGENTS tab is missing from the drawer | The server was started without `--agents`. The tab is hidden, not empty, in that case | Restart with `--agents <file>`; check `systemctl --user cat eunice` to see the unit's actual `ExecStart` |
| An agent never fires | It is `enabled = false`; or the schedule genuinely has not come due; or the server was restarted before each occurrence | Expand the card and read "next run"; check the startup log for `watching N enabled agent(s)` and the per-agent line |
| The scheduler logged `no enabled agents, scheduler idle` | Every agent in the file is disabled, or the file has zero `[[agent]]` tables (which is valid) | Set `enabled = true` and restart |
| An agent fires on the wrong day | Almost always a day-of-week misunderstanding | Compare your expression with the `(as "...")` form in the startup log. Remember `0` and `7` are both Sunday, and that eunice shifts numbers by one internally, so `1-5` correctly appears as `2-6` |
| The run fires at the wrong hour | The server's local timezone differs from yours, or DST moved | Check `timedatectl` on the server host; the schedule is evaluated in the server's local time |
| Ticks are recorded as `skipped` (yellow dot) | The previous run was still in flight | Widen the interval, cut the prompt's workload, or accept it — nothing queues up |
| A run fails with an auth error under systemd but works fine in your shell | The service does not inherit your shell environment; the key was exported after the last `--install`, or rotated since | Export the key, re-run `eunice --install`, and check that it prints `Captured N API key(s)` rather than the no-keys warning. `~/.eunice/eunice.env` lists exactly what the service will see |
| An agent's Bash tool reports "command not found" for something on your `PATH` | Same cause: the service uses the `PATH` captured at install time | Re-run `eunice --install` from a shell with the right `PATH`, or use absolute paths in the prompt |
| Edits to `agents.toml` or a `prompt_file` have no effect | The file is invalid, so the previous config is still running | Check the drawer or the log for the rejection message; `journalctl --user -u eunice \| grep "reload REJECTED"`. Reloads take a few seconds; `systemctl --user reload eunice` forces one |
| A browser save is refused with "changed on disk" | The file was edited elsewhere since the editor was opened | Use the modal's reload button, then save again. Your typing is preserved |
| A run is marked failed with `timed out after Ns` | The work exceeded `timeout_secs` | Raise `timeout_secs`, or narrow the prompt. The partial transcript is in the session |
| A run is marked `timed out after Ns and did not stop cleanly` | It was stuck in a long tool call or API request and could not be interrupted within the 30-second grace window | Add a timeout to the command the prompt runs; the transcript for that run is gone, only the failure note remains |
| The AGENTS tab shows "never run" after a restart | Run state is in-memory and rebuilt on restart | The runs themselves are still in SESSIONS; the tab repopulates on the next fire |
| `systemctl --user start eunice` fails immediately, or the service restarts every 5 seconds | Usually the port is already taken, or the binary named in `ExecStart` no longer exists | `journalctl --user -u eunice -n 50` shows the real error. `ss -tlnp \| grep <port>` finds a port conflict; `systemctl --user cat eunice` shows the path it is trying to run |
| `Failed to start eunice.service: Unit eunice.service not found` | Never installed, or installed for a different user | `eunice --install` as the user who will run it — user units are per-user, and `sudo systemctl --user` is not the same thing |
| The service does not come back after a reboot | Lingering was not enabled — the installer warns but does not fail when the system refuses | `loginctl enable-linger "$USER"`, then confirm with `loginctl show-user "$USER" --property=Linger` |
| It works from your shell but the unit runs an old build | `ExecStart` points at the binary you ran `--install` with, and a running service keeps the old binary until restarted | `systemctl --user cat eunice` to see the path; re-run `--install` using `which eunice`, and restart after any `eunice --update` |
| `sessions.db` is not where you expected | It is created in `WorkingDirectory`, which is whatever directory you ran `--install` from | `systemctl --user show eunice --property=WorkingDirectory`; `cd` to the directory you want and re-run `--install` |

### Reading the logs

The scheduler writes to stdout, which under systemd means the journal:

```bash
journalctl --user -u eunice -f                 # follow
journalctl --user -u eunice --since today      # today only
journalctl --user -u eunice | grep scheduler   # scheduler lines only
```

Every scheduler line is prefixed `[HH:MM:SS.mmm] [scheduler]`, and per-run lines add the agent name:

```
[09:00:00.007] [scheduler] [morning-digest] run starting in session 7c1e0a44 (neon-wintermute)
[09:01:12.884] [scheduler] [morning-digest] run complete
[09:30:00.003] [scheduler] [repo-watch] previous run still in flight, skipping this tick
[10:00:00.011] [scheduler] [health-check] run failed: timed out after 120s
```

---

## 9. Worked examples

### A morning repo digest

Summarizes yesterday's activity in a repository and leaves a file behind. `working_dir` means the
prompt can talk about the repo without spelling out paths.

```toml
[[agent]]
name = "morning-digest"
schedule = "0 8 * * 1-5"
working_dir = "/home/me/p/myrepo"
timeout_secs = 900
prompt = """
Run `git log --since=yesterday.midnight --until=today.midnight --stat` in this repository.
Summarize what changed: which areas of the codebase moved, anything that looks risky, and any
commit that touches more than 10 files.
Write the summary to digests/$(date +%F).md, creating the digests directory if needed.
If there were no commits, write a single line saying so.
"""
```

### A periodic health check that writes to a file

Short timeout, frequent schedule, cheap model. The prompt is deliberately narrow so the run finishes
well inside its window.

```toml
[[agent]]
name = "health-check"
schedule = "*/15 * * * *"
model = "flash"
timeout_secs = 120
prompt = """
Check that https://example.com/healthz returns HTTP 200 using curl with a 10 second timeout.
Also check that disk usage on / is below 90%.
Append one line to /home/me/health.log in the form: "<ISO timestamp> OK" or
"<ISO timestamp> FAIL <what failed>".
Do not write anything else.
"""
```

### A weekday-only end-of-day report

Uses `prompt_file` so the prompt can be edited and version-controlled separately. Remember the path
is relative to the `agents.toml` directory, and that changing it needs a restart.

```toml
[[agent]]
name = "eod-report"
schedule = "30 17 * * MON-FRI"
prompt_file = "prompts/eod-report.md"
model = "sonnet"
working_dir = "/home/me/p/myrepo"
timeout_secs = 1200
```

### A weekly cleanup, parked for now

Disabled agents still appear in the UI, which makes `enabled = false` a better staging area than a
commented-out block.

```toml
[[agent]]
name = "weekly-cleanup"
schedule = "0 3 * * 0"
enabled = false
prompt = """
List files under /home/me/scratch that have not been modified in more than 30 days.
Write the list to /home/me/scratch-cleanup-candidates.txt. Do not delete anything.
"""
```

### Putting it together

A complete `agents.toml` is just those tables concatenated into one file. Install it as a service and
watch the first runs go by:

```bash
eunice --install \
  --port 8811 \
  --model sonnet \
  --agents /home/me/agents/agents.toml

journalctl --user -u eunice -f
```

Then open <http://localhost:8811> and check the AGENTS tab.

---

## See also

- `README.md` — the Long-Running Agents overview and the full CLI reference
- `docs/superpowers/specs/2026-07-20-long-running-agents-design.md` — design rationale, alternatives
  considered, and the reasoning behind the cron translation
