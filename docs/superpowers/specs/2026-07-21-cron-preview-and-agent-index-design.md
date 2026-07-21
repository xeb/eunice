# Live cron preview, full-screen agent index, and save-to-index flow

**Date:** 2026-07-21
**Status:** Approved, ready for implementation
**Builds on:** `2026-07-21-agent-editor-routing-design.md` (v1.0.7)

## Problem

Three gaps in the agent editor introduced in v1.0.7:

1. **Cron is opaque.** The SCHEDULE field takes a raw 5-field expression with a static hint.
   Nothing tells the user what it means or when it will next fire, and mistakes surface only
   when a run does or does not happen.
2. **`#/agent/` is not a page.** It parses to `segments = ['agent']`, falls through to the chat
   view, and is passed as `session: 'agent'` — i.e. it tries to open a session literally named
   "agent". There is no full-screen way to see the agent list.
3. **Save exits sideways.** Saving returns to the chat view and slides the drawer open, which is
   a jarring destination for an action taken on a full-page form.

## Part A — Live cron preview

### A1. Backend: expose the server timezone

Cron fires in the **server's** local time; JS runs in the **browser's**. On a remote agent host
these differ and a client-computed "next run" would be confidently wrong.

- Add `server_timezone` (IANA name, e.g. `America/Los_Angeles`) to the `/api/agents` response
  (`AgentsResponse` in `src/webapp/scheduler.rs`).
- Source it from `iana-time-zone`, which is **already in the dependency tree** via chrono, so
  promoting it to a direct dependency adds no new compilation.
- If the IANA name cannot be determined, the field is `null` and the client falls back to
  browser-local with an explicit "assuming your local time" note rather than lying.

### A2. Frontend: cron module + live hint

A small vanilla-JS module in `webapp/index.html`:

- `parseCron(expr)` — validates the 5 fields, returns field sets or a precise, field-named
  error (`"Hour must be 0-23 (got 25)"`, not "invalid cron")
- `describeCron(expr)` — plain English (`"At 09:00, Monday through Friday"`)
- `nextRuns(expr, fromInstant, timeZone, n)` — next fire times as absolute instants

**It must match server semantics, not Unix semantics:**

- Day-of-month and day-of-week are **intersected**, not unioned. `0 9 1 * 1` fires only on a
  Monday that is also the 1st. This is the `cron` crate's behavior; see
  `agents::restricts_both_day_fields` and its comment at `agents.rs:100-105`.
- Input uses **Unix DOW numbering** (0=Sunday), which `agents::normalize_cron` translates to the
  crate's 1=Sunday. The JS accepts what the user types, i.e. Unix numbering, plus day names.

Rendered live under the SCHEDULE field:

```
✓ At 09:00, Monday through Friday
  Next run in 14h 23m (Tue Jul 22, 09:00 server time)
```

Invalid input shows the specific field error. An expression restricting both day fields also
shows the intersection warning, which today reaches only the server log:

```
⚠ Unix cron would fire on either. This fires only when both match.
```

**Timezone math is the highest-risk piece.** Computing next-fire in a non-browser timezone must
handle DST gaps and overlaps. Use `Intl.DateTimeFormat` with an explicit `timeZone` to read
server-local wall-clock fields, and resolve wall-clock back to an instant via an offset lookup
rather than assuming a fixed offset.

**Search must be bounded.** Naive minute-stepping hangs on sparse expressions — the next
`0 0 29 2 *` can be years away. Step field-by-field (year → month → day → hour → minute) with a
cap of ~5 years, and report "no run in the next 5 years" rather than spin.

**Parity caveat:** the JS validator and the Rust one may disagree at the edges. The server
remains the authority — save already validates and surfaces errors — so a disagreement degrades
to "preview said ✓, save failed with a clear message", never a bad write.

## Part B — `#/agent/` full-screen agent index

New route: `segments === ['agent']` → `view: 'agentIndex'`. This also removes the latent
`session: 'agent'` misparse described above.

A full-screen, full-width flat list carrying the same data as the drawer's AGENTS tab: status
dot, name, schedule, model, next run, last run. Clicking a row opens `#/agent/<name>`. A
`+ NEW AGENT` button sits in the page header.

Using `describeCron` from Part A, each row shows the plain-English schedule alongside the raw
expression, so the index is readable without decoding cron.

The drawer's AGENTS tab stays exactly as it is; this is an additional surface, not a
replacement.

## Part C — Save returns to the index with a fading confirmation

Saving currently navigates to the chat view and opens the drawer. Instead:

- Save → navigate to `#/agent/` and show a transient toast: `Saved "burn-construct"`
- Delete → same, with `Deleted "burn-construct"`
- The toast fades after ~3s, carries `role="status"` / `aria-live="polite"`, and respects
  `prefers-reduced-motion` (no animation, still auto-dismisses)

For coherence, **`← AGENTS` and CANCEL also redirect to the index** rather than chat-plus-drawer.
The back link finally has a real destination.

The §3 dirty-form discard guard from the previous spec continues to front every exit path,
including these new ones.

## Testing

Rust: `server_timezone` presence and shape in the `/api/agents` payload, following the existing
serialization-shape test pattern in `scheduler.rs`.

Frontend has no test harness, so the cron module gets manual verification against a live server,
driving a real browser. Cases that must be checked because they are where this breaks:

- DST spring-forward and fall-back boundaries in a server timezone that observes DST
- A server timezone deliberately different from the browser's
- Sparse expressions (`0 0 29 2 *`) completing without hanging
- Day-of-month ∩ day-of-week producing the same next-fire the server would compute
- Field-level error messages for each of the five fields
