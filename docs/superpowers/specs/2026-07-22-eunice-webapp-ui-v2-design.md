# Eunice Webapp UI v2 — Design Spec

**Date:** 2026-07-22
**Branch:** `eunice-xeb-ai-deploy`
**Target version:** 1.0.9
**Deploy target:** `eunice.xeb.ai` (this machine) + full public release

## Goal

Wire the approved static mock (`~/x/ideas/eunice_ui2.html`, "UI v2 · sacristy gilt")
into the real embedded webapp UI (`webapp/index.html`), replacing the current
drawer-based dark UI with the mock's top-tab "cockpit" shell and gilt light/dark
theme — **without losing any existing functionality**. Then bump to 1.0.9, deploy to
the live `eunice.xeb.ai` systemd service, and cut a full public release.

## Constraints

- **Rust is untouched.** The UI is a single embedded file served via
  `include_str!("../../webapp/index.html")` in `src/webapp/handlers.rs`. No routes,
  handlers, APIs, or types change. This spec rewrites exactly one file: `webapp/index.html`.
- **No feature regression.** Every capability in the current UI must survive the reskin:
  SSE streaming chat, markdown rendering, tool cards, thinking indicator, session
  history restore, live event reconnect, session list + delete, the full agent editor
  (name/schedule/prompt/model/timeout/working-dir/enabled), the client-side cron engine
  (preview + next-runs + timezone/DST handling), save/delete/toggle/reload, confirm
  dialog, and toasts.
- **The cron engine stays byte-for-byte where possible.** The ~500-line client-side cron
  parser/next-run engine (`cronParseField` … `nextRuns` … `renderCronPreview`) is logic,
  not chrome. Preserve it; only its *preview markup* may be restyled.
- **Loopback + Access stay load-bearing.** Deploy must keep `127.0.0.1:8812` bind and the
  Cloudflare Access 302. No change to systemd unit, tunnel, DNS, or Access policy.

## Source & target inventory

**Mock** (`~/x/ideas/eunice_ui2.html`, 447 lines): sticky topbar (brand + Chat/Sessions/
Agents tabs + status LED + model chip + theme toggle + `+ New`); hash router over three
`.view` sections; sacristy-gilt light/dark tokens with `data-theme` + `prefers-color-scheme`;
IBM Plex Mono/Sans; graph-paper grid background; chat thread with `.msg`/`.tool`/`.thinking`/
`.composer`; sessions `.list`/`.row`; agent `.card` with the signature 24-hour `.strip`.

**Real UI** (`webapp/index.html`, 4651 lines): `<style>` ~11–1738; markdown-it CDN; drawer
(`#session-drawer`) + overlay + hamburger; agent index page (`#agent-index-page`); agent
editor page (`#agent-page`); confirm dialog; toast; chat container (header/messages/input/
status-bar); ~950 lines of app JS incl. SSE, session logic, and the cron engine.

## Architecture (component mapping)

| Concern | Current | v2 |
|---|---|---|
| Shell nav | Slide-out drawer + hamburger | Sticky top cockpit bar, 3 tabs |
| View switching | Drawer tabs + full-page agent overlays | Hash router over `.view` sections (`#/chat`, `#/sessions`, `#/agents`) + existing `#/agent/<name>`, `#/agent/new` |
| Theme | Single (dark terminal) | Gilt light/dark via `data-theme`, honors `prefers-color-scheme`, persisted to `localStorage` |
| Fonts | System | IBM Plex Mono/Sans (Google Fonts CDN) |
| Chat | header + `#messages` + input-area | `.thread` of `.msg`, `.tool` cards, `.thinking`, sticky `.composer` |
| Sessions | Drawer list | Full-page routed `.view` with filter box |
| Agents | Drawer list + `#agent-index-page` | Full-page `.view` of `.card`s each with a 24h `.strip` |
| Agent editor | `#agent-page` overlay | Same routes, restyled to gilt tokens |
| Status | header badges | Topbar LED + model chip bound to `/api/status` + `/api/config` |

### Data flow (unchanged endpoints)

- **Chat:** `POST /api/query` (SSE) → `handleEvent`/`processSSEData`; `POST /api/cancel`;
  restore via `POST /api/session/history`; live watch via `POST /api/session/events`.
- **Sessions:** `GET /api/sessions`; `POST /api/session/new|delete|clear`.
- **Agents:** `GET /api/agents`; `POST /api/agents/get|save|delete|reload`.
- **Status:** `GET /api/status`, `GET /api/config`.

The reskin rewires DOM selectors and rendering functions (`renderSessionItem`,
`renderAgentCard`, message/tool builders) to the new markup. Fetch calls, payload shapes,
SSE parsing, and the cron engine are preserved.

### The 24-hour cron strip (signature component)

The mock hardcodes each agent's `hour`/`minute`. In v2 these are **derived from the real
schedule**: reuse the existing `nextRuns(expr, fromInstant, timeZone, count)` to get the next
fire, and place the mark at `(localHour + localMinute/60)/24 * 100%`. A live "now" line uses
the browser clock. Agents disabled or without a parseable next run render the strip without a
mark (or omit it), never throw. Multiple daily fires: show the next fire's mark (documented
as a known simplification; the card's cron text carries the full expression).

## Error handling

- Preserve all existing error paths (fetch failures, SSE errors, cancel, confirm dialogs,
  agent form validation, cron parse errors → inline preview error, not a throw).
- Theme toggle and strip rendering must be defensive: a bad/empty cron never breaks the
  Agents view; a missing `/api/config` field falls back to a neutral model chip.
- `prefers-reduced-motion` disables animations (carried from the mock).

## Testing & verification

1. `cargo test` — all 328 tests pass (UI is an embedded string; no Rust logic changes, but
   run to confirm nothing regressed and the build embeds the new file).
2. Local run on a scratch port (e.g. `--no-persist --port 8899`), then:
   - Chat: send a prompt, confirm SSE stream, a tool card, markdown, thinking indicator.
   - Sessions tab: list renders from `/api/sessions`, filter works, row opens in Chat.
   - Agents tab: cards render, 24h strip marks match schedules, card opens editor.
   - Editor: cron preview + next-runs still compute; save/toggle/delete/reload work.
   - Theme toggle flips light/dark and persists; mobile widths (≤760, ≤400) hold.
3. Screenshot the running app in light and dark for a visual record.
4. After deploy: `curl -s 127.0.0.1:8812/api/status` shows `1.0.9`; `ss -tlnp | grep 8812`
   is `127.0.0.1` only; `curl -sI https://eunice.xeb.ai` is `302` → Access.

## Version, deploy, release

Per `CLAUDE.md` release steps:

1. `cargo test` green.
2. Update LOC + release binary size in `README.md`.
3. Bump `Cargo.toml` `version = "1.0.9"`.
4. Commit on `eunice-xeb-ai-deploy` with a descriptive message.
5. **Deploy to eunice.xeb.ai (this machine):** `cargo install --path . --force` →
   `systemctl --user restart eunice.service` → verify (served version, loopback bind,
   Access 302).
6. **Full public release:** push the branch; update `~/p/longrunningagents.com/version.txt`
   to `1.0.9` and `wrangler pages deploy ~/p/longrunningagents.com
   --project-name=longrunningagents --commit-dirty=true --branch=master`.
7. `eunice --update` verifies the published version resolves to 1.0.9.

## Out of scope (YAGNI)

- No changes to the systemd unit, cloudflared config, DNS, or Access policy.
- No new API endpoints; no server-side rendering.
- No new agent features — editor parity only.
- No offline/self-hosted fonts (CDN is acceptable; app already uses a CDN for markdown-it).
