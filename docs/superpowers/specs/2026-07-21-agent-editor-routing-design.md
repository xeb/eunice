# URL-addressable agent editor + agent-run session separation

**Date:** 2026-07-21
**Status:** Approved, ready for implementation

## Problem

Three issues in `--webapp` mode:

1. **The agent editor is a modal.** Creating or editing an agent has no URL, so it cannot be
   linked, bookmarked, or refreshed, and the browser back button does nothing. The PROMPT
   textarea is also capped at 400px inside a centered card, which is tight for real agent
   prompts.
2. **Agent runs are indistinguishable from interactive chats.** They are tagged in the data
   (`sessions.agent_name`) but presented in the same chronological list with only an 11px ⏰
   emoji and a hover tooltip. Because every scheduled run creates a *new* session, a frequent
   agent buries the user's own chats.
3. **Unclear whether agents inherit the default system prompt.** (Resolved — see Non-Goals.)

## Non-Goals

**System prompt inheritance is already correct and is not changing.** `run_agent_with_events`
prepends `state.system_prompt` whenever conversation history is empty
(`handlers.rs:1380-1386` via `compose_first_message`), and every scheduled run starts a fresh
empty session, so every agent run already receives the configured system prompt ahead of its
own prompt. This is the desired behavior. No `agents.toml` field is added to control it.

That decision is load-bearing for compatibility: `AgentsFile` is `#[serde(deny_unknown_fields)]`
(`agents.rs:33`), so adding any config field would make older eunice binaries hard-reject a
newer `agents.toml`. Adding none keeps configs bidirectionally compatible.

## Design

### 1. Routing

The hash router grows from one route to three. `updateSessionUrl` writes `#/<session-name>`,
and generated session names are two-word slugs (`sprawl-molly`) that never contain `/`, so
segment count disambiguates cleanly.

| URL | View |
|---|---|
| `#/<session-name>` | chat (existing, unchanged) |
| `#/agent/<name>` | agent editor, editing `<name>` |
| `#/agent/new` | agent editor, creating |

A single `applyRoute()` reads the hash and shows exactly one of `.container` / `#agent-page`.
It runs on load and on `hashchange`, so deep links, refresh, and back/forward all resolve
through one code path.

Degenerate deep links resolve rather than dead-end:

- Unknown agent name → "Agent not found" state with a back link
- Server started without `--agents`, or `/api/agents` reports `editable: false` → redirect to
  chat with the drawer open on the AGENTS tab

### 2. Full-page editor

The form moves out of `#agent-modal` into a new `#agent-page` element, a sibling of
`.container`. Every existing field, hint, fingerprint check, and validation path is preserved
as-is. Changes are structural only:

- Header: `← AGENTS` back link plus `EDIT AGENT: <name>` / `NEW AGENT` title
- The PROMPT textarea becomes the page's flex-grow element (min ~40vh) instead of a
  400px-capped box. This is the actual motivation for the full page.
- The action bar keeps DELETE / RELOAD / CANCEL / SAVE, and **keeps the existing arm-twice
  DELETE pattern** (`.armed` / `disarmAgentDelete`) rather than routing it through the new
  confirm dialog — it already works and suits a destructive action.
- The mobile breakpoint's `.modal` rules become `.agent-page` rules

`showAgentEditor()` stops being the entry point; navigation is. The EDIT and `+ NEW AGENT`
buttons set the hash, and the router fetches `/api/agents/get` and renders.

### 3. Discard-unsaved confirm dialog

A generic `#confirm-dialog` reusing the `.modal-overlay` / `.modal` CSS freed up by moving the
editor to a page. Exposed as `confirmDialog({title, message, confirmLabel})` returning a
Promise. No `window.confirm`.

**Back-button handling is the subtle part.** A custom dialog is asynchronous and therefore
cannot block a `hashchange` the way `window.confirm` blocks synchronously. On a dirty-form
`hashchange`:

1. Immediately restore the previous hash, keeping the editor mounted
2. `await` the confirm dialog
3. Only then apply the pending route

A re-entrancy guard is required so the programmatic hash restore does not re-trigger the
handler.

The one remaining `window.confirm` (session delete, `index.html:2253`) is converted to the
same dialog so no raw browser dialog is left in the app.

### 4. Sessions list: grouped sections

`/api/sessions` already returns `agent_name`, so grouping is client-side: **CHATS**
(null `agent_name`) then **AGENT RUNS**, each sorted by `updated_at` descending.

Section headers render **only when both groups are non-empty**. With no agents configured the
list renders identically to today — the same principle the drawer tabs already follow
(`index.html:584`).

Agent-run rows get an accent left stripe, the agent name as the primary line, and session name
+ turn count + relative time as meta, plus the run-status mark from §5. The ⏰ emoji is
removed. No cap on the runs section; the drawer already scrolls.

### 5. Run-status persistence

The only non-cosmetic change. Run status currently lives only in the scheduler's in-memory
`AgentRunState` (`scheduler.rs:59-61`) — one `last_status` *per agent*, not per session, lost
on restart. `SessionMetadata` has no status field.

- Add `sessions.run_status TEXT` in `migrate_schema`, mirroring the `agent_name` precedent
  exactly (`persistence.rs:189`)
- Values: `running` / `success` / `failed`. `skipped` never creates a session, so it is not
  a valid stored value.
- Written as `running` at `create_agent_session`; updated at the `finish_run` call sites
  (`scheduler.rs:832`, `:840`). A timeout lands as `failed`.
- **Startup sweep:** any row still at `running` at startup is rewritten to `interrupted` —
  otherwise a server killed mid-run leaves a permanently-spinning row. Rendered with its own
  distinct mark.
- Added to `SessionMetadata`, so it flows to `/api/sessions` *and* the AGENTS tab's
  `recent_sessions` for free
- Mirrored on `MemorySession` so `--no-persist` behaves identically

### 6. Backwards compatibility

Verified, both directions:

- **Old DB + new binary:** `migrate_schema` adds the nullable column via `ALTER TABLE`.
  Pre-existing rows read `NULL` and render with no status mark.
- **New DB + old binary:** no `SELECT *` exists anywhere in `persistence.rs`; every query names
  its columns explicitly, so the extra column is ignored.
- **Configs:** no `agents.toml` field is added or changed (see Non-Goals).

## Testing

Rust tests follow the existing patterns in `persistence.rs`:

- Migration on an old database, mirroring `test_migrate_schema_adds_agent_name_to_old_database`
- Run-status round-trip for both the SQLite and Memory backends
- The `running` → `interrupted` startup sweep
- `SessionMetadata` serialization shape

The frontend has no test harness, so routing, the dirty-navigation guard, and list grouping get
manual verification against a live `--webapp --agents` server.
