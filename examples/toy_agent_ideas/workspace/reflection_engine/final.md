# Agent: The Reflection Engine

## One-Liner
A "Quantified Self" agent for software engineers that mines personal git history to build a predictive model of the user's error patterns, providing real-time, personalized preventative coaching.

## Problem Domain
Generic linters (ESLint, Pylint) catch syntactic errors but miss *habitual logic errors* specific to a developer's psychology (e.g., "I always forget to await async calls in Python" or "I tend to introduce race conditions in this specific module"). Developers often repeat the same class of mistake for years without realizing the pattern.

## Core Toolset
- **memory:** To build a persistent Knowledge Graph of the user's coding habits, error frequencies, and successful fix strategies.
- **shell:** To mine historical data (`git log -p`, `git blame`) and run local analysis.
- **filesystem:** To watch for real-time file changes and provide "Sidecar" feedback.
- **grep:** To identify structural patterns in code that match historical error templates.

## Architecture

### 1. The Mining Loop (Offline/Batch)
- **Frequency:** Weekly or on-demand.
- **Action:**
  - The agent scans the git history for "Fix" commits (identified by keywords: "fix", "bug", "revert", "oops").
  - It extracts the *diff*: What was removed (the error) vs. What was added (the fix).
  - It generalizes the "removed" code into a structural pattern (e.g., replacing concrete values with wildcards).
  - **Memory Update:** Creates/Updates `ErrorPattern` nodes in the graph. Links them to specific `Modules` or `Languages`.
  - *Example:* "User removed `x = []` inside a loop and moved it outside." -> Inference: "Variable Scope / Initialization Error".

### 2. The Shadow Loop (Real-Time)
- **Frequency:** Continuous (inotify/fswatch).
- **Action:**
  - As the user writes code, the agent matches the current AST/Text against the database of `ErrorPattern` nodes.
  - It calculates a "Risk Score" based on context (e.g., "This is a `Network` module, and you have a high error rate with `Timeouts` in network code").
  - **Intervention:** It **does not** modify the code. It updates a specialized markdown file (`REFLECTION.md`) or CLI dashboard.
  - *Message:* "💡 Pattern Detected: You are initializing a list inside a loop. You fixed this same bug in commit `a1b2c3` 3 months ago."

## Persistence Strategy (Hybrid)
- **Memory Graph:** Stores the high-level relations (User -> tendency -> ErrorType).
- **Filesystem:** Stores the raw pattern library (Regex/AST selectors) and the "Sidecar" UI files.

## Human Interface
- **The "Sidecar" File:** A markdown file that the agent keeps updated. The user keeps it open in a split pane. It acts as a dynamic "Coach's Notebook".
- **Explicit Feedback:** The user can add comments to the sidecar: "<!-- IGNORE: This is intentional -->". The agent reads this and updates the Memory Graph to reduce false positives.

## Failure Modes & Recovery
1. **Pattern Overfitting:** The agent flags every `for` loop as an error.
   - *Recovery:* The agent tracks the "Ignore Rate". If a pattern is ignored > 80% of the time, it is deprecated automatically.
2. **Performance Impact:** Analyzing every keystroke is slow.
   - *Recovery:* The agent uses a "Debounce" strategy and only analyzes changed lines, not full files.

## Novelty / Key Insight
Most AI coding agents try to *write* the code for you. The Reflection Engine tries to *improve the writer* by treating their past mistakes as a personalized training dataset. It is "Metacognition as a Service".
