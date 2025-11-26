# Component: Display (UX Layer)
**Path:** `src/display.rs`
**Last Analyzed:** 2025-11-26 13:49
**Primary Author(s):** Mark Kockerbeck
**Lines:** 210

## Purpose
Provides all user-facing terminal output: spinners for async operations, formatted tool call/result display, model listings, and debug output. Acts as the sole presentation layer, keeping UI logic isolated from business logic.

## Structure
Single file with clear functional groupings:

| Section | Lines | Description |
|---------|-------|-------------|
| Imports | 1-9 | External deps: `colored`, `indicatif`, `std::sync::atomic` |
| `Spinner` | 11-33 | Generic loading spinner with message |
| `ThinkingSpinner` | 35-77 | Elapsed-time counter for LLM thinking |
| `print_tool_*` | 80-102 | Tool call/result formatting |
| `print_model_info` | 104-112 | Model display |
| `print_mcp_info` | 114-134 | MCP server summary |
| `print_dmn_mode` | 136-139 | DMN indicator |
| `print_error` / `debug` | 141-151 | Error/verbose output |
| `print_model_list` | 153-186 | Full model listing |
| `get_key_status` | 188-210 | API key status helper |

## Key Patterns

### 1. Braille Spinner Animation
```rust
.tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
```
Unicode braille characters create smooth 10-frame animation at 80ms intervals.

### 2. Atomic Flag for Async Spinners
```rust
running: Arc<AtomicBool>
```
`ThinkingSpinner` uses atomic bool + spawned task for non-blocking elapsed time updates. Clean shutdown via `Ordering::Relaxed` store.

### 3. Consistent Emoji Vocabulary
| Emoji | Meaning |
|-------|---------|
| 🔧 | Tool call |
| → | Tool result |
| 🔌 | MCP servers header |
| 📡 | Individual MCP server |
| 🧠 | DMN mode |
| ❌ | Error |
| ✅ | Available |
| 🤖/💎/🧠/🦙 | Provider icons (OpenAI/Gemini/Anthropic/Ollama) |

### 4. Output Truncation
```rust
if limit > 0 && lines.len() > limit {
    // show first N lines + "...X more lines"
}
```
Tool results are line-limited to prevent terminal flooding.

### 5. Dimmed Secondary Info
Arguments, remaining line counts, and verbose messages use `.dimmed()` for visual hierarchy.

### 6. API Key Masking
```rust
format!("...{}", &key[key.len() - 4..])  // Shows only last 4 chars
```
Security-conscious display of key status without exposing full key.

## Dependencies

**Internal:**
- `crate::models::Provider` - Provider enum and `get_icon()`
- `crate::provider::get_available_models` - Model enumeration

**External (Cargo.toml):**
- `colored = "2"` - Terminal color/styling (`.red()`, `.bold()`, `.dimmed()`)
- `indicatif = "0.17"` - Progress bars and spinners

**Standard Library:**
- `std::env` - API key lookup
- `std::sync::atomic` - Async spinner control
- `std::time::Duration` - Tick intervals

## Consumers

| File | Functions Used |
|------|---------------|
| `agent.rs` | `Spinner`, `ThinkingSpinner`, `debug`, `print_tool_call`, `print_tool_result` |
| `interactive.rs` | `print_model_info`, `print_mcp_info`, `print_dmn_mode`, `print_error` |
| `main.rs` | `print_model_list`, `print_model_info`, `print_mcp_info`, `print_dmn_mode` |

## Concerns

**None significant.** This is a clean, well-isolated presentation module.

Minor observations:
1. **Duplicated startup prints** - Both `interactive.rs` and `main.rs` call similar print sequences; could consolidate into a single `print_startup_info()` function.
2. **No tests** - Display code is hard to test, but snapshot tests could verify output format.
3. **Hardcoded colors** - No theming support (expected for CLI tool).

## Git History

```
c9f6694 v0.1.10: Add thinking indicator with elapsed time
10aff86 v0.1.8: Gemini 3 Pro refactors
c0ce5d9 Reduce codebase to 1,950 lines and add Gemini native API support
bf1cbbe v0.1.1 bump
a3c2b17 Migrate everything to Rust
```
The `ThinkingSpinner` with elapsed time was a recent addition (v0.1.10).

## Notes

- The file demonstrates good separation of concerns: all terminal output routing through one module.
- Emoji usage is intentional UX design, not decoration - each emoji has consistent semantic meaning.
- The dual spinner types (`Spinner` vs `ThinkingSpinner`) reflect different UX needs: generic "working" vs "LLM is thinking with time awareness".
- Provider icons are defined in `models.rs`, not here, maintaining single source of truth.
