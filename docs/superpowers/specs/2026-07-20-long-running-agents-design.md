# Long-Running Agents for Eunice Webapp Mode — Design Spec

**Date:** 2026-07-20
**Status:** Implemented
**Target version:** 1.1.0

> Revised after implementation. Changes from the original draft are marked
> **[revised]** and explained where they appear.

## Overview

Add scheduled, long-running agents to eunice's webapp mode. Agents are defined in a
plain-text TOML file, run on cron schedules inside the webapp server process, and
write their transcripts into the existing session store so they are viewable in the
web UI. A new `--install` flag installs the webapp as a systemd **user** service
(no sudo) bound to a port chosen at install time.

## Goals

- Define agents declaratively in an external `agents.toml` file passed as a CLI argument.
- Run each agent on a cron schedule while the webapp server is up.
- Each run produces a normal webapp session (full transcript, tools, persistence).
- Read-only "Agents" view in the webapp hamburger drawer: schedule, status, run history.
- `eunice --install` sets up a systemd user-mode service (`systemctl --user`), enabled
  at boot via lingering, bound to the port given at install time.

## Non-Goals

- ~~Editing agents from the web UI. The TOML file is the single source of truth.~~
  **[superseded]** — see "Hot Reload and Editing" below.
- ~~Hot-reload of `agents.toml` (restart the service to pick up changes; SIGHUP reload
  is a possible future enhancement).~~ **[superseded]** — same.
- Catch-up/backfill of missed schedules while the server was down.
- Multi-user auth, remote management, or agent-to-agent orchestration.

## Hot Reload and Editing (added after the original spec)

Both of the first two non-goals were reversed on request. The design that replaced them:

**Reload.** A watcher fingerprints `agents.toml` plus every file it references via `prompt_file`
and polls every 3s; `SIGHUP` (wired to systemd `ExecReload`) forces one. A change is parsed and
validated on a blocking thread — `detect_provider` may make a blocking Ollama probe — and swapped in
only on success.

The governing rule inverts startup behavior: **at startup an invalid config aborts; once running, an
invalid config is rejected and the previous one keeps serving.** A typo in a file being edited must
never take down a daemon that is already working. The rejection is recorded and surfaced through
`/api/agents` so the UI can explain why an edit did not apply, and the fingerprint of the rejected
bytes is stored so the same failure is not re-logged every poll.

Across a swap, run state is preserved for agents that still exist by name and dropped for those that
do not; per-agent `Client`s and `ToolRegistry`s are reused unless the model or `working_dir` actually
changed. In-flight runs are never cancelled — only future scheduling reflects the new config — and
`finish_run` is a no-op for an agent that no longer exists, so a run outliving its own deletion
cannot resurrect a phantom state entry. `last_tick` is not reset, so a newly added agent does not
back-fire historical occurrences.

Two early exits had to be removed for this to work: the scheduler previously returned when zero
agents were enabled, and when no future occurrence existed. Either would have permanently wedged
the loop against a later reload.

**Editing.** The AGENTS tab gained create/edit/enable/delete. Writes go through `toml_edit` rather
than re-serializing, so comments, key order and formatting in a hand-written file survive a save.
The pipeline validates the *proposed* file text before writing anything, then writes atomically
(temp file + rename) with a `.bak`. Each edit carries the fingerprint the editor was opened with;
a mismatch is refused as a conflict rather than clobbering a concurrent edit. Renaming is not
supported, because run state and `sessions.agent_name` are keyed by name.

**Accepted risk.** Editing is deliberately ungated: no auth gate, no host restriction. The webapp
has no authentication of its own, so anyone who can reach the port can create scheduled shell
execution. This was an explicit product decision; the docs steer users to `--host 127.0.0.1`.

## Alternatives Considered

1. **In-process tokio scheduler (chosen).** A background task inside the `--webapp`
   process parses cron expressions and fires runs directly into `SessionStorage`.
   Pros: runs share the server's client/tool registry/session DB; live status in the UI;
   one process to manage. Cons: schedules only fire while the server runs (acceptable —
   the daemon install makes the server always-on).
2. **User crontab entries invoking `eunice` one-shot.** Rejected: results would not land
   in `sessions.db`, no UI visibility, and crontab mutation is fragile to manage.
3. **systemd timer units per agent.** Rejected: requires regenerating unit files on every
   config change and scatters state across systemd; same visibility problem as (2).

## Agent Configuration File

### Format

TOML, loaded from the path given by the new `--agents <file>` flag:

```toml
# agents.toml
[[agent]]
name = "daily-digest"            # required, unique, [a-z0-9-]+
schedule = "0 9 * * *"           # required, 5-field cron (min hour dom mon dow), local time
model = "sonnet"                 # optional; defaults to the server's model
prompt = "Summarize yesterday's git activity in ~/p/myrepo and write it to digest.md"

[[agent]]
name = "repo-watch"
schedule = "*/30 * * * *"
prompt_file = "prompts/repo-watch.md"   # alternative to prompt; relative to agents.toml's directory
enabled = true                   # optional, default true; disabled agents still show in the UI
timeout_secs = 600               # optional, default 600; run is aborted and marked failed on expiry
working_dir = "/home/xeb/p/myrepo"  # optional; cwd for Bash/Read/Write during the run
```

Field rules:

- `name`: required, unique across the file, kebab-case (`^[a-z0-9][a-z0-9-]*$`), max 64 chars.
- `schedule`: required, standard 5-field cron expression evaluated in the server's local
  timezone.

  **[revised] Cron translation.** The `cron` crate is not standard-cron compatible: it rejects
  5-field expressions (it wants a leading seconds field) and it numbers day-of-week
  1=Sunday…7=Saturday, whereas Unix cron uses 0=Sunday…6=Saturday. Taking the crate's dialect
  directly would have made `0 9 * * 1` fire on Sunday instead of Monday. `agents.toml` therefore
  accepts **standard 5-field Unix cron only**, and `agents::normalize_cron` rewrites it into the
  crate's 6-field form, remapping bare day-of-week integers (including inside lists, ranges and
  steps) and leaving `*` and names like `MON-FRI` alone. Both the original and translated
  expressions are logged at startup so an unexpected firing day is diagnosable.
- Exactly one of `prompt` / `prompt_file` is required. `prompt_file` paths resolve
  relative to the directory containing `agents.toml`; the file is read at startup.
- `model`: optional. Validated at startup via `detect_provider()`; unknown model = startup error.
- `enabled`: optional bool, default `true`.
- `timeout_secs`: optional u64, default 600.
- `working_dir`: optional; must exist at startup. When set, the run's tool execution
  uses it as the working directory; otherwise the server's cwd is used.

  **[revised] How this is implemented.** A process-wide `chdir` would race with concurrent
  interactive queries, so instead the tools gained an optional cwd: `BashTool` passes it to
  `Command::current_dir`, and `ReadTool`/`WriteTool` resolve relative paths against it. Each
  agent with a `working_dir` gets its own `ToolRegistry` built once at startup; agents without
  one share the server's registry and behave exactly as before.

### Validation (fail fast)

All validation happens at server startup, before binding the port. Any failure prints a
message naming the agent and field, then exits non-zero:

- unreadable/unparseable TOML (with the TOML error's line info)
- duplicate or invalid `name`
- invalid cron expression
- both or neither of `prompt`/`prompt_file`; missing `prompt_file`
- unknown `model`
- nonexistent `working_dir`

An empty agent list (file exists, zero `[[agent]]` tables) is valid — the server runs
with no scheduled agents and the UI shows an empty Agents view.

## Scheduler

New module: `src/webapp/scheduler.rs`.

- Spawned as a tokio task from `run_server()` when `--agents` was provided.
- Loop: compute the earliest `next_run` across enabled agents (via the `cron` crate's
  upcoming-occurrence iterator), `tokio::time::sleep` until then, fire due agents, repeat.
- **Concurrency:** at most one in-flight run per agent. If an agent is still running when
  its next tick arrives, that tick is recorded as `skipped` and the schedule moves on.
  Different agents may run concurrently.
- **A run:**
  1. Create a new session via `SessionStorage::create_agent_session`, tagged with the agent's
     name. **[revised]** The session keeps its normal generated name rather than a
     `⏰ <name> — <date>` title: `sessions.name` is `NOT NULL UNIQUE`, so a date-derived title
     would collide when two runs land in the same minute. The `agent_name` column carries the
     association instead, and the UI renders the ⏰ badge from it.
  2. **[revised]** Execute the *webapp's* agent loop, `handlers::run_agent_with_events`. The
     webapp does not use `agent::run_agent` — it has its own loop that emits SSE events. Reusing
     it means a scheduled run behaves identically to an interactive one, and (because the run
     publishes a broadcast channel and marks the session running) a browser can attach to an
     in-flight scheduled run through the existing `/api/session/events` endpoint.
  3. Persist every message to the session as it is produced (same path as interactive
     webapp queries).
  4. On completion, record `success`; on error or timeout, record `failed` with the error
     string appended to the session as a final assistant message.
     **[revised]** `run_agent_with_events` returns `Result<(), String>` so the scheduler can tell
     a failed run from a successful one; previously it returned `()` and swallowed errors into an
     SSE event, which would have recorded every failure as a success.
- Runs are independent of interactive traffic; they do not touch `cancel_tx` and cannot
  be cancelled from the UI in v1.
- In-memory per-agent state (not persisted; rebuilt on restart):
  `{ next_run_at, last_run_at, last_status: running|success|failed|skipped, last_session_id }`.
  History older than the last run is discoverable through the tagged sessions themselves.

## Persistence Changes

`src/webapp/persistence.rs`:

- Add nullable `agent_name TEXT` column to the sessions table. Migration: on open, if
  the column is missing, `ALTER TABLE sessions ADD COLUMN agent_name TEXT`.
- `list_sessions` includes `agent_name` so the UI can badge/filter agent-run sessions.
- New query: list sessions for a given `agent_name`, newest first, limit N (for the
  Agents detail view).

## CLI Changes (`src/main.rs`)

New flags:

```
--agents <file>       Path to agents.toml (webapp mode only; error if used without
                      --webapp/--install)
--install             Install eunice --webapp as a systemd user service and start it
--uninstall-service   Stop, disable, and remove the systemd user service
```

`--install` composes with existing flags: `--port`, `--host`, `--model`, `--agents`,
`--prompt`, `--no-persist` are captured into the generated unit's `ExecStart`.

Example:

```
eunice --install --port 9000 --agents /home/xeb/agents/agents.toml --model sonnet
```

## Daemon Install (systemd user mode)

New module: `src/daemon.rs`. Behavior of `eunice --install`:

1. **Validate first.** If `--agents` was given, parse and validate the file exactly as
   the server would; refuse to install a service that would crash-loop.
2. **Resolve absolute paths:** the current executable (`std::env::current_exe()`), the
   agents file, and the working directory (cwd at install time — this is where
   `sessions.db` will live).
3. **Snapshot API keys.** systemd user services do not inherit the login shell's
   environment, so the installer writes any of these vars currently set to
   `~/.eunice/eunice.env` with mode `0600`:
   `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `GOOGLE_API_KEY`,
   `OLLAMA_HOST`. Existing file is overwritten on reinstall.
4. **Write the unit** to `~/.config/systemd/user/eunice.service`:

   ```ini
   [Unit]
   Description=Eunice agentic webapp server
   After=network-online.target

   [Service]
   Type=simple
   ExecStart=/abs/path/to/eunice --webapp --port 9000 --host 0.0.0.0 --agents /abs/path/agents.toml
   WorkingDirectory=/abs/path/of/install-cwd
   EnvironmentFile=-%h/.eunice/eunice.env
   Restart=on-failure
   RestartSec=5

   [Install]
   WantedBy=default.target
   ```

5. **Enable and start:** `systemctl --user daemon-reload`, then
   `systemctl --user enable --now eunice.service`.
6. **Enable lingering** so the service runs at boot without a login session:
   `loginctl enable-linger <user>`. If this fails (some distros gate it behind polkit),
   print a warning with the manual command rather than failing the install.
7. Print status: unit path, URL (`http://host:port`), and the follow-logs command
   (`journalctl --user -u eunice -f`).

`eunice --uninstall-service`: `systemctl --user disable --now eunice.service`, delete the
unit file, `daemon-reload`. Leaves `~/.eunice/eunice.env`, `sessions.db`, and lingering
untouched (print a note about each). Reinstalling over an existing unit overwrites it and
restarts the service.

Non-Linux platforms: `--install`/`--uninstall-service` print a clear "systemd user
services require Linux" error and exit non-zero.

## Web UI: Agents View

### API

New endpoint `GET /api/agents` returning:

```json
{
  "agents_file": "/home/xeb/agents/agents.toml",
  "agents": [
    {
      "name": "daily-digest",
      "schedule": "0 9 * * *",
      "model": "sonnet",
      "enabled": true,
      "prompt_preview": "Summarize yesterday's git activity in...",
      "next_run_at": "2026-07-21T09:00:00-07:00",
      "last_run": {
        "at": "2026-07-20T09:00:00-07:00",
        "status": "success",
        "session_id": "abc123"
      },
      "recent_sessions": [{ "id": "abc123", "title": "⏰ daily-digest — 2026-07-20 09:00" }]
    }
  ]
}
```

`prompt_preview` is the first ~200 chars of the resolved prompt. When the server was
started without `--agents`, the endpoint returns `{"agents_file": null, "agents": []}`.

### UI (webapp/index.html)

- The hamburger drawer gains a two-tab header: **SESSIONS** (current behavior) and
  **AGENTS**. The AGENTS tab is hidden entirely when `agents_file` is null.
- AGENTS tab shows one card per agent: name, status dot (green = last run success,
  red = failed, gray = never run/disabled, pulsing = running), cron string, model,
  "next run in …" relative time, and last-run relative time.
- Tapping a card expands it in place: prompt preview (read-only) and the recent run
  sessions as links — clicking one loads that session transcript in the main chat view,
  exactly like clicking a session in the SESSIONS tab.
- No create/edit/delete controls anywhere. A footer note reads
  "Agents are configured in <agents_file> — restart the service to apply changes."
- Agent-run sessions in the SESSIONS tab get a small ⏰ badge (driven by `agent_name`).
- The list refreshes on drawer open and every 30 s while the AGENTS tab is visible.

## New Dependency

- Add `cron` and `chrono` to Cargo.toml — cron expression parsing and next-occurrence
  computation in local time.

## Error Handling Summary

| Failure | Behavior |
|---|---|
| Bad agents.toml at startup/install | Exit non-zero with agent + field named |
| Agent run errors or hits timeout | Run marked `failed`; error appended to the session |
| Previous run still active at next tick | Tick marked `skipped`, logged |
| Server restart | In-memory run state resets; sessions remain in sessions.db |
| `enable-linger` denied | Warning + manual command, install still succeeds |
| `--install` on non-Linux | Clear error, exit non-zero |

## Testing

Following the existing in-module `#[cfg(test)]` convention:

- **Config parsing:** valid file; duplicate names; bad cron; both/neither prompt fields;
  unknown model; `prompt_file` resolution relative to the TOML's directory; empty list.
- **Scheduler logic:** next-run computation across agents; skip-when-running behavior
  (pure-function tests on the state machine, no timers).
- **Persistence:** `agent_name` column migration on an existing DB; tagged session
  listing and per-agent queries.
- **Daemon:** unit-file generation is a pure string function — assert ExecStart,
  WorkingDirectory, EnvironmentFile contents; env-snapshot writes only set vars.
- **API:** `/api/agents` serialization with and without `--agents`.
- **CLI:** flag parsing (`--agents` requires webapp/install; `--install` composition).

## File Changes

```
src/main.rs                  --agents / --install / --uninstall-service flags, wiring
src/daemon.rs                NEW: unit generation, env snapshot, systemctl/loginctl calls
src/webapp/scheduler.rs      NEW: agent config types, TOML loading/validation, cron loop
src/webapp/server.rs         AppState gains agent registry + run-state map; spawn scheduler
src/webapp/handlers.rs       GET /api/agents
src/webapp/persistence.rs    agent_name column + migration + queries
webapp/index.html            AGENTS tab in drawer, session badges
Cargo.toml                   + cron
CLAUDE.md / README.md        docs, LOC/binary size update
```
