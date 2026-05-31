# Eunice TUI Enhancements — Richer `--chat` / `--tui` (Claude-style)

Make the interactive experience look and feel like the Claude Code terminal UI:
inverse-bar user turns, a framed input that grows as you type, and a live, colored
"thinking" line. Reference capture: `CLAUDE_UI_EXAMPLE.png`.

> `--chat` and `--tui` are the same mode (`run_tui_mode` in `src/tui/app.rs`). The non-TTY
> fallback is `src/interactive.rs`. This plan targets both and ultimately unifies them.

---

## ✅ Implemented (2026-05-29) — default framed editor

Phase 0 + 1 + a pragmatic Phase 2 are **implemented and the framed editor is the default** `--chat`/`--tui`.
`cargo test` green (254 tests: 152 bin + 102 lib, incl. new `theme` + `frame_editor` geometry tests);
release build clean (no warnings).

- **`src/theme.rs`** (new) — centralized palette + helpers (`user_bar`, `rule_with_width`, `thinking_line`, `footer`), with unit tests for width math. Declared in both crate roots (`lib.rs` + `main.rs`).
- **`src/tui/frame_editor.rs`** (new) — the bordered, vertically-expanding input box (top rule + right `eunice` label, `›` prompt, bottom rule, footer), redrawing per keystroke. Ports the proven raw-mode editing/history logic from `interactive.rs`.
- **`src/tui/app.rs`** — `run_tui_mode` now defaults to `run_tui_framed`; the old r3bl path is kept as `run_tui_classic` behind **`EUNICE_TUI_CLASSIC=1`** (safety fallback). Submitted turns render as a full-width **inverse bar**; thinking shows in **accent coral**.
- **`src/display_sink.rs`** — accent thinking line + asterisk spinner; also fixed a pre-existing bug where tool results printed a literal `\n` instead of newlines.

**Design choice (deviation from the §3 plan, deliberate):** rather than a DECSTBM scroll region with output pinned below a fixed frame, the frame is drawn **during input only**; during generation, output streams normally via a raw-mode-safe writer (`RawStdoutWriter`, `\n`→`\r\n`) and the framed box reappears at the bottom for the next turn. This drops the off-by-one/resize risks the panel flagged and is far more robust, at the cost of the box not staying pinned mid-stream. If you want true bottom-pinned-during-stream, that's the DECSTBM/Approach-A escalation.

> ⚠️ **Verification:** this was built headless — it compiles and unit-tests pass, but the live rendering
> (box drawing, expansion, raw-mode I/O, Esc-cancel) **needs eyeballing in a real terminal**. If anything
> looks off, `EUNICE_TUI_CLASSIC=1 eunice --chat` reverts to the known-good r3bl path while we iterate.

---

## 1. Target aesthetic (from the screenshot)

```
 If you'd rather I launch it and monitor the build/download to completion …      ← assistant: plain text
 ✻ Churned for 43s                                                                ← prior status (dim, accent glyph)
███ can you also make a helper flag that is an alias? --gemma … ███████████████   ← USER turn: full-width INVERSE bar, bold
 ✻ Precipitating… (12s · ↓ 379 tokens · thinking with xhigh effort)              ← THINKING: accent color + spinner + live meta
 ⎿ Tip: You have free guest passes to share · /passes                            ← connector sub-line (dim)
 ──────────────────────────────────────────────────────────────── ultracode –   ← top rule + right-aligned MODE label
 › ▍                                                                              ← INPUT (expands downward as you type)
 ────────────────────────────────────────────────────────────────────────────   ← bottom rule
 ▸▸ bypass permissions on (shift+tab to cycle) · esc to interrupt                 ← footer hint (accent)
```

Three requested elements, plus the supporting chrome:

| # | Element | Behavior |
|---|---|---|
| 1 | **User turn** | Submitted user text is redrawn as a **full-width inverse/black bar**, bold. |
| 2 | **Input frame** | Two horizontal rules with the prompt between them; **expands vertically** as the buffer wraps; a **right-aligned mode label** sits on the top rule; a **footer hint** sits below. |
| 3 | **Thinking line** | Distinct **accent color**, animated spinner glyph, **live metadata** (elapsed · tokens · effort), and an indented **tree-connector** sub-line. |
| — | Assistant text | Plain (terminal default fg). |
| — | Streaming | Model output keeps flowing **above** the fixed input frame. |

### Color palette (themeable; uses *relative* SGR so it adapts to the user's terminal theme)

| Role | SGR / 256-color | Notes |
|---|---|---|
| User bar | `\x1b[7m\x1b[1m` (reverse + bold), space-padded to full width | reverse adapts to any bg (the "black" in the shot is the green theme inverted) |
| Thinking accent | `\x1b[38;5;209m` (warm coral) | spinner glyph + status text |
| Connector sub-line | `\x1b[2m` (dim) | `⎿`/`└` + tip |
| Mode label | `\x1b[38;5;141m` (purple — eunice's existing `PURPLE`) | right-aligned on top rule |
| Footer hint | `\x1b[38;5;211m` (pink) | mode + keybindings |
| Rules | `\x1b[2m` `─`×width | dim horizontal lines |
| Spinner cycle | `✻ ✶ ✺ ✹ ✷` (asterisk/sparkle) | Claude-style; replaces current braille+emoji mix |

Centralize all of this in a new `src/tui/theme.rs` (constants + helpers like `rule(width)`,
`user_bar(text, width)`, `accent(s)`), so the two display sinks stop hardcoding duplicate ANSI.

---

## 2. Current state (what we're building on)

- **`src/tui/app.rs`** — `--chat`/`--tui` via `r3bl_tui` `ReadlineAsyncContext` (single `> ` prompt;
  `SharedWriter` prints output *above* the prompt). User input is echoed by r3bl; the submitted line
  is not re-styled. Cancel keys handled by a separate `monitor_cancel_keys` task (raw-mode `event::read`).
- **`src/interactive.rs`** — non-TTY fallback: a **hand-rolled crossterm raw-mode editor**
  (`read_line_with_history` / `redraw_line` / `move_cursor_to`) with history, slash autocomplete, and
  multiline-wrap cursor math (lines ~360-362). This is the seed for the framed editor.
- **`src/display_sink.rs`** — `DisplayEvent` enum + `StdDisplaySink` (indicatif spinner) and
  `TuiDisplaySink` (`SharedWriter` + manual ANSI). Colors hardcoded per sink; `ThinkingStart` carries
  **no metadata**.
- **`src/agent.rs`** — emits `ThinkingStart`/`Stop`, `StreamChunk`/`End`, `ToolCall`, `ToolResult`,
  `Response`, `Info`, `Error`; tracks `session_usage`; the streaming closure already sees token counts.

---

## 3. Architecture decision — phased **C → B**, with **A** as the escape hatch

Three approaches were evaluated against the actual code:

| Criterion | A — full `r3bl_tui` App rewrite | **B — custom crossterm editor + scroll region** | C — `readline_async` + decorations |
|---|---|---|---|
| (1) Inverse user bar | ✅ | ✅ padded `\x1b[7m\x1b[1m` | ✅ via `should_print_line_on(false,false)` + bar |
| (2) **Expanding framed input** + label + footer | ✅ (manual measure each frame) | ✅ `render_frame` recomputes height/keystroke; DECSTBM anchors it | ❌ **impossible** — r3bl renders input as one logical line, no below-buffer hook |
| (3) Live thinking line | ✅ | ✅ ticker repaints reserved band | ✅ ticker writes via SharedWriter |
| Delivers **all three**? | Yes | **Yes** | **No** (fails #2 — the headline) |
| Streaming above fixed frame | Cleanest (diffed buffer) | Good (DECSTBM scroll region + one write mutex) | Frame isn't truly fixed/below |
| Effort / Risk / Blast radius | very-high / high / largest | high / high / medium-large | medium / medium / smallest (3 files) |
| Resize mid-stream | framework-handled | cosmetic scrollback corruption | inherits r3bl for input |

**Decision:**
- **C alone cannot meet the brief** — `ReadlineAsyncContext` renders input as a single logical line and
  exposes no hook to draw a bottom rule/footer beneath a growing buffer. Shipping only C silently drops
  requirement #2 (the headline element).
- **B is the only *incremental* path that delivers all three**, because it grows the editor that already
  works in `interactive.rs`. DECSTBM scroll regions (`\x1b[{top};{bottom}r`) are the textbook primitive
  for "fixed bottom band, output scrolls above" — exactly the streaming-coexistence requirement.
- **A delivers all three most cleanly** but is a near-total rewrite with the largest blast radius — not
  justified as a *starting* point when B reuses proven code. **A is the explicit escape hatch** if resize
  corruption (B's one real weakness) proves intolerable.

**Why phased, not "just B":** Phase 1 (C) ships *two of three* elements fast on the lowest-risk surface,
and forces us to land the prerequisites **B also needs** — the `DisplayEvent` metadata extension, the
agent-side elapsed/token/effort plumbing, and the hand-rolled ticking spinner. Phase 2 (B) then only has
to solve the genuinely hard part (DECSTBM-anchored expanding frame + write serialization) on top of an
already-validated metadata flow, with a shippable milestone in between.

---

## 4. Build order (file-level)

### Phase 0 — Shared foundation (prerequisite for C *and* B)

- **`src/display_sink.rs`** — make thinking carry data:
  ```rust
  pub enum DisplayEvent {
      ThinkingStart { verb: String, effort: Option<String> }, // was a unit variant
      ThinkingTick  { elapsed_secs: u64, tokens: u64 },        // NEW
      ThinkingStop,
      UserMessage   { text: String },                          // NEW (render as inverse bar)
      // …existing variants unchanged…
  }
  ```
  `StdDisplaySink` can ignore the new fields (indicatif already shows a spinner); add the bar to its
  `UserMessage` arm too for consistency in non-TUI single-shot runs.
- **`src/agent.rs`** — at each `ThinkingStart` emission (≈lines 150, 199) pass `verb` (e.g. a rotating
  "Thinking/Precipitating/Churning") and `effort` (from model/config). Start an `Instant` at thinking
  start and emit `ThinkingTick { elapsed_secs, tokens }` from the streaming closure (≈158-167, where the
  chunk/usage counts already exist) on a coarse cadence. **Keep `run_agent_cancellable`'s signature and
  the watch-channel cancel contract unchanged.**
- **`src/tui/theme.rs`** (NEW) — the palette + helpers from §1; both sinks consume it.

### Phase 1 — Decorations on r3bl (Approach C) — *ships user bar + thinking line*

- **`src/tui/app.rs`**
  - After `try_new` (≈131): `ctx.readline.should_print_line_on(false, false)` to suppress r3bl's echo.
  - Add `print_user_bar(sw, text, cols)` → full-width `\x1b[7m\x1b[1m{text:<cols}\x1b[0m` (`cols` from
    `crossterm::terminal::size()`); call it on `ReadlineEvent::Line` (≈175) and the `initial_prompt`
    path (≈155), replacing the bare `> {text}` echo.
  - Top of the loop (≈172): print the top rule + right-aligned mode label; print the footer hint after.
- **`src/display_sink.rs`** — `TuiDisplaySink`: on `ThinkingStart`, spawn a tokio `interval(120ms)` task
  holding a cloned `SharedWriter` + `Arc<AtomicU64>` (tokens) + `Arc<AtomicBool>` (stop) + start `Instant`.
  Each tick erases its prior 2 lines (`\x1b[2K` / `\x1b[1A`) and rewrites the accent status line +
  dimmed `⎿` connector. `ThinkingStop` (or first `StreamChunk`) sets the AtomicBool and aborts the task.
  **Do not** use r3bl's `Spinner` (it strips ANSI; ColorWheel/Braille only).

*Milestone: inverse user bars + live colored thinking line. Input is still r3bl's single-line prompt,
now with a top rule + mode label above it.*

### Phase 2 — True expanding frame (Approach B) — *delivers requirement #2*

- **`src/tui/input_editor.rs`** (NEW) — migrate `read_line_with_history` / `redraw_line` /
  `move_cursor_to` / `show_suggestions` out of `interactive.rs` into an `InputEditor` struct. Add:
  - `render_frame()` — recompute `rows = max(1, ceil((prompt_len + display_width)/term_width))` (cap ~10);
    `frame_height = rows + 2` (rules); draw top rule + right label, the wrapped buffer with `›`, the
    bottom rule + footer; reposition the cursor. **Recompute geometry from scratch each repaint** (no
    incremental cursor math).
  - `commit_user_turn()` — erase the frame, emit the inverse bar into the scroll region, re-arm.
  - Use **`unicode-width`** for display width (not the byte/char count at current `interactive.rs:360-362`)
    or wide/CJK chars misalign the bar and frame.
- **`src/display_sink.rs`** — replace `TuiDisplaySink` with `FrameDisplaySink` holding
  `Arc<Mutex<FrameState>>` (`{ stdout, frame_top_row, frame_height, thinking_height, label,
  status_metadata, spinner_phase }`). **Every** write — agent stream, ticker, frame repaint — goes through
  this one mutex (this serialization replaces `SharedWriter`). Set the DECSTBM scroll region
  `\x1b[1;{frame_top_row-1}r`; on grow, emit `\n` at the scroll-region bottom, then re-anchor
  `frame_top_row = term_height - frame_height - thinking_height` and reset DECSTBM.
- **`src/tui/app.rs`** — drop `ReadlineAsyncContext` / `SharedWriter` / `choose` / `show_command_menu`;
  loop on `InputEditor::read_line`; route `process_prompt` through `FrameDisplaySink`. **Delete
  `monitor_cancel_keys`** — the editor's single poll loop now reads Esc / double-Ctrl+C during generation
  and forwards to the existing `watch::Sender<bool>` (two stdin readers cannot coexist; see Risk 1).
- **`src/interactive.rs`** — reduce to a thin wrapper over the shared `InputEditor` (or keep as the
  non-TTY fallback), unifying both code paths and deleting duplicated color constants.
- **`src/tui/mod.rs`** — `mod input_editor; mod theme;` and fix the module doc that claims r3bl usage.

---

## 5. Top risks & mitigations

1. **Keyboard contention (two stdin readers).** Once `InputEditor` owns raw mode (Phase 2), a separate
   `event::read()` task (`app.rs:312 monitor_cancel_keys`, `interactive.rs:551 monitor_escape_key`) will
   drop/duplicate keys. **Mitigation:** delete the monitor task; the editor's single poll loop reads
   Esc/Ctrl+C during generation and forwards to the `watch::Sender<bool>`. Land this **first** in Phase 2
   and explicitly test *cancel-during-stream*. (Latent in Phase 1 too — keep the monitor output-free and
   only active during generation, when r3bl isn't polling.)
2. **Cursor/scroll-region off-by-one on grow.** When the buffer wraps to a new row, scroll history up
   (`\n` at scroll-region bottom) *before* re-anchoring `frame_top_row` and resetting DECSTBM, or the band
   smears into the transcript. **Mitigation:** centralize all geometry in `FrameState` behind the one
   mutex; recompute in `render_frame` from scratch; add a manual test matrix for grow/shrink at
   terminal-height boundaries.
3. **Resize mid-stream corruption (B's inherent limit).** SIGWINCH can't reflow output already scrolled
   under the old geometry. **Mitigation:** on resize, recompute DECSTBM + fully repaint the band/frame
   (fixes future frames; transient scrollback artifacts accepted). **Persistent intolerable corruption is
   the explicit trigger to escalate to Approach A** — keep the Phase-0 metadata/sink seam clean so an A
   migration reuses the agent + `DisplayEvent` work instead of redoing it.

---

## 6. Testing

- **Unit:** `render_frame` height math (1-line, wrapped, capped); `user_bar` padding at various widths and
  with wide/CJK chars; thinking-line formatting (`elapsed`/`tokens`/`effort`).
- **Manual matrix:** type past one line (grow), delete back (shrink), submit (bar + frame re-arm), stream a
  long reply (output stays above frame), resize mid-stream, cancel mid-stream (Esc / double Ctrl+C), `/`
  command menu, history up/down, non-TTY pipe (fallback path).
- **Regression:** single-shot (non-chat) output unchanged; `StdDisplaySink` still fine when piped.

## 7. Out of scope (future)

- **Approach A** full `r3bl_tui` rewrite (only if resize corruption forces it).
- Markdown / syntax-highlighted code blocks in assistant output (a separate renderer pass).
- Mouse, scrollback search, theme auto-detection (light vs dark), user-configurable palette.

---

### Effort summary
Phase 0 ≈ small (enum + agent plumbing + theme). Phase 1 ≈ medium, ships 2/3 elements on the lowest-risk
surface. Phase 2 ≈ the bulk (new `input_editor.rs` + `FrameDisplaySink` + scroll-region bookkeeping),
delivers the expanding framed input. A is held in reserve.
