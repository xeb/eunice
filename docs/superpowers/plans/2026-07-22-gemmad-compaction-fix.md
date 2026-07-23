# Task 8 — Fix gemmad context-window overflow (auto-compaction not firing)

> **For agentic workers:** implement via a fresh subagent with TDD, then a task review. Steps use checkbox syntax.

**Reported symptom:** `API error: API request failed with status 400: {"error":{... "message":"request does not fit the context window: prompt is 1209570 tokens + 0 reasoning budget; n_ctx is 32768. Shorten the prompt or lower reasoning_effort." ...}}` on `eunice.xeb.ai` (webapp + gemmad). User: "as if the autocompression in eunice isn't working."

## Root cause (investigated, confirmed)

Three compounding faults; the first is the proximate cause:

1. **`is_context_exhausted_error()` (`src/compact.rs:335`) does not recognize gemmad/llama.cpp's wording.** The message contains "request does not fit the context window … n_ctx is 32768" — none of the matcher's substrings hit: it has no "context length" (only "context window"), no "limit"/"exceed"/"too long"/"resource_exhausted". So the matcher returns **false**, the reactive compaction branch in the webapp loop (`src/webapp/handlers.rs:1519`) is never entered, and the raw 400 surfaces to the UI. **This is why "autocompression isn't working."**

2. **No proactive compaction anywhere.** Both loops (`handlers::run_agent_with_events`, `agent::run_agent`) compact ONLY reactively, in the API-error handler. `conversation_history` accumulates across turns and is persisted per session (`set_history`), growing unbounded — here to 1.2M tokens — with nothing trimming it before the send.

3. **`compact_context()` is not token-budget-aware, so even if it fired it could fail.** Its hard-trim (`src/compact.rs:232`) triggers on message **count** (`HARD_TRIM_THRESHOLD = 500`), not tokens, and keeps a fixed 50 recent messages with no token check — 50 large messages can still blow a 32k window. Its full-summarization fallback (`generate_summary`, `src/compact.rs:168`) sends the **entire** formatted history to the same 32k model, which 400s identically. Nothing ever targets the model's actual `n_ctx`.

## Fix strategy

Make the **reactive** path actually work and self-configure to the model's real window, so the user never sees the error (auto-compaction retries transparently). Keep it DRY in `src/compact.rs`; wire both loops.

- **A.** Teach `is_context_exhausted_error` the llama.cpp/gemmad phrasing.
- **B.** Parse the real window from the error (`n_ctx is N`) and trim the history to fit that budget **without a model round-trip**, guaranteeing a fitting prompt.
- **C.** Wire both agent loops to use A+B: on a context error, parse `n_ctx`, trim to a safe fraction of it, retry.

Proactive pre-send compaction is a noted follow-up (see "Out of scope") — not needed to stop the error, since after B the compacted (shrunk) history is what gets saved to the session, so it does not immediately re-bloat.

## Global constraints
- Repo `/media/xeb/GreyArea/projects/eunice`, branch `eunice-xeb-ai-deploy`. TDD: failing test first.
- `estimate_tokens` is a rough `len/4` heuristic and underestimates; the trim target must leave HEADROOM for the system prompt, tool schemas, and the model's reply. Target = `floor(n_ctx * 0.6)`, and never below a floor of 2000 tokens.
- Do not change the summarization prompt or the OpenAI/Gemini/Anthropic matcher substrings that already exist — only ADD to them.
- `cargo test` must stay green (328+ tests).

---

### Task 8: context-window-aware reactive compaction

**Files:**
- Modify: `src/compact.rs` (matcher, a parser, a token-budget trim; tests)
- Modify: `src/webapp/handlers.rs` (reactive branch ~line 1519 uses the parsed window + trim)
- Modify: `src/agent.rs` (reactive branch ~line 301 — parity)

- [ ] **Step 1 (RED): matcher test**

Add to `src/compact.rs` tests:
```rust
#[test]
fn test_gemmad_context_window_error_is_detected() {
    let msg = "API request failed with status 400: {\"error\":{\"code\":null,\"message\":\"request does not fit the context window: prompt is 1209570 tokens + 0 reasoning budget; n_ctx is 32768. Shorten the prompt or lower reasoning_effort.\",\"param\":null,\"type\":\"invalid_request_error\"}}";
    assert!(is_context_exhausted_error(msg));
    assert!(!is_rate_limit_error(msg));
}
```
Run `cargo test test_gemmad_context_window_error_is_detected` → FAILS (matcher returns false).

- [ ] **Step 2 (GREEN): extend the matcher**

In `is_context_exhausted_error` (`src/compact.rs:335`), add these OR-clauses (do not remove existing ones):
```rust
        || error_lower.contains("does not fit the context window")
        || error_lower.contains("context window")
        || error_lower.contains("n_ctx")
```
Run the Step-1 test → PASS. Run `cargo test` (compact module) → all green.

- [ ] **Step 3 (RED): context-window parser test**

```rust
#[test]
fn test_parse_context_window_from_error() {
    let msg = "request does not fit the context window: prompt is 1209570 tokens + 0 reasoning budget; n_ctx is 32768. Shorten the prompt";
    assert_eq!(parse_context_window(msg), Some(32768));
    assert_eq!(parse_context_window("some unrelated error"), None);
}
```
Run → FAILS (no `parse_context_window`).

- [ ] **Step 4 (GREEN): add `parse_context_window`**

Add to `src/compact.rs` (pub):
```rust
/// Extract the model's context window from a llama.cpp/gemmad overflow error,
/// e.g. "... n_ctx is 32768. ...". Returns None if not present.
pub fn parse_context_window(error_msg: &str) -> Option<usize> {
    let idx = error_msg.find("n_ctx is ")?;
    let after = &error_msg[idx + "n_ctx is ".len()..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<usize>().ok().filter(|&n| n > 0)
}
```
Run Step-3 test → PASS.

- [ ] **Step 5 (RED): token-budget trim test**

```rust
#[test]
fn test_trim_to_token_budget_fits() {
    // 200 messages of ~1000 tokens each (~200k tokens) trimmed to a 32768 window.
    let big = "x".repeat(4000); // ~1000 tokens at 4 chars/token
    let mut messages = Vec::new();
    for _ in 0..200 {
        messages.push(Message::User { content: big.clone() });
        messages.push(Message::Assistant { content: Some(big.clone()), tool_calls: None });
    }
    let target = (32768.0 * 0.6) as usize; // ~19660
    let trimmed = trim_to_token_budget(&messages, target);
    assert!(estimate_tokens(&trimmed) <= target, "trimmed still exceeds target");
    assert!(!trimmed.is_empty());
    // The most recent message is preserved.
    assert!(matches!(trimmed.last(), Some(Message::Assistant { .. })));
}
```
Run → FAILS (no `trim_to_token_budget`).

- [ ] **Step 6 (GREEN): add `trim_to_token_budget`**

Add to `src/compact.rs` (pub). Keep the newest messages that fit under the budget (after first applying lightweight tool-output truncation), drop the oldest, and prepend a one-line context note. No model call.
```rust
/// Trim history to fit a hard token budget WITHOUT calling the model.
/// Keeps the most recent messages that fit under `target_tokens` (after
/// lightweight tool-output truncation) and prepends a note. Guarantees the
/// result estimates at or under `target_tokens` (or a single note+message
/// when even one message is too large).
pub fn trim_to_token_budget(messages: &[Message], target_tokens: usize) -> Vec<Message> {
    let cfg = CompactionConfig::default();
    let light = lightweight_compact(messages, &cfg);
    let note = Message::User {
        content: "## Context Note\n\n[Earlier conversation was trimmed to fit the model's context window.]".to_string(),
    };
    let note_tokens = estimate_tokens(std::slice::from_ref(&note));
    let budget = target_tokens.saturating_sub(note_tokens);

    let mut kept: Vec<Message> = Vec::new();
    let mut used = 0usize;
    for msg in light.iter().rev() {
        let t = estimate_tokens(std::slice::from_ref(msg));
        if used + t > budget && !kept.is_empty() {
            break;
        }
        used += t;
        kept.push(msg.clone());
    }
    kept.reverse();
    let mut out = vec![note];
    out.extend(kept);
    out
}
```
Run Step-5 test → PASS. Run `cargo test` (compact module) → all green.

- [ ] **Step 7: wire the webapp reactive branch**

In `src/webapp/handlers.rs` at the context-exhausted branch (~1519), replace the single-shot `compact_context` call so it uses the parsed window and the guaranteed trim. Read the current block first; change it to:
```rust
                if is_context_exhausted_error(&error_msg) && !compaction_attempted {
                    log(&format!("[{}] Context exhausted, compacting to fit window...", log_prefix));
                    let target = crate::compact::parse_context_window(&error_msg)
                        .map(|n| ((n as f64) * 0.6) as usize)
                        .unwrap_or(0);
                    let compacted_msgs = if target >= 2000 {
                        crate::compact::trim_to_token_budget(&conversation_history, target)
                    } else {
                        // Unknown window: fall back to the existing strategy.
                        match compact_context(client, &provider_info.resolved_model, &conversation_history, &compaction_config).await {
                            Ok(c) => c.messages,
                            Err(compact_err) => {
                                log(&format!("[{}] Compaction failed: {:#}", log_prefix, compact_err));
                                conversation_history.clone()
                            }
                        }
                    };
                    conversation_history = compacted_msgs;
                    compaction_attempted = true;
                    log(&format!("[{}] Compacted to ~{} tokens", log_prefix, crate::compact::estimate_tokens(&conversation_history)));
                    continue; // retry with the fitting context
                }
```
(Confirm `estimate_tokens` is `pub` — it is. Keep the existing `use crate::compact::{...}`; add `estimate_tokens`/`parse_context_window`/`trim_to_token_budget` to it or fully-qualify as above.)

- [ ] **Step 8: wire the CLI reactive branch (parity)**

In `src/agent.rs` at the reactive `compact_context` call (~301), apply the same parse-window-then-trim logic (fully-qualified `crate::compact::parse_context_window` / `trim_to_token_budget`, target = `n_ctx * 0.6`, floor 2000, else fall back to `compact_context`). Read the surrounding block first to match its variable names (`conversation_history`, `config`, `model`).

- [ ] **Step 9: full suite + build**

Run `cargo test 2>&1 | tail -15` — all green. Run `cargo build --release 2>&1 | tail -3` — succeeds.

- [ ] **Step 10: commit**

```bash
git add src/compact.rs src/webapp/handlers.rs src/agent.rs
git commit -m "Fix gemmad context-window overflow: recognize llama.cpp error, trim to n_ctx

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

## Out of scope (follow-up, not needed to stop the error)
- **Proactive pre-send compaction:** remember the learned `n_ctx` per session/client and trim *before* the request when `estimate_tokens` exceeds budget, so not even one round-trip fails. Needs a source of the window before the first error (learned-and-remembered, or a `--context-window`/gemmad default). Deferred.
- Replacing the `len/4` token heuristic with a real tokenizer.
