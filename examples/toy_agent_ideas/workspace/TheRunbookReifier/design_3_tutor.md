# Design 3: The Shell Tutor (Interactive)

## Purpose
To standardize team knowledge and prevent "knowledge silos" by offering real-time suggestions based on the team's collective "Golden Path" runbooks.

## Loop Structure
1. **Interception**: (Simulated) The agent scans `.bash_history` immediately after commands are executed (via `PROMPT_COMMAND` hook writing to a shared file).
2. **Analysis**: It parses the last command (e.g., `kubectl get pods | grep error`).
3. **Lookup**: It checks the `memory` graph for a "Better Way" or "Alias" (e.g., "Use `k_errors` alias instead").
4. **Feedback**: It writes a tip to the terminal (or a `tips.txt` sidecar file being watched by another pane).
5. **Mining**: If a user does something *new* and complex that succeeds (no subsequent error code), the agent asks: "That looked useful. Save as runbook?"

## Tool Usage
*   **shell**: Hooks into history, maybe uses `notify-send` for tips.
*   **memory**: Stores the "Canon" of best practices and aliases.
*   **filesystem**: Reads shared team config/docs to learn the "Canon".

## Memory Architecture
*   **Nodes**: `Pattern`, `BestPractice`, `AntiPattern`.
*   **Relations**: `Pattern -> is_deprecated_by -> BestPractice`.
*   **Example**: `"rm -rf /" -> is_dangerous -> "Use safe_rm"`.

## Failure Modes
*   **Annoyance**: Clippy-like behavior ("I see you're trying to list files..."). **Mitigation**: High threshold for interruption. Only suggest for complex/dangerous commands.
*   **Lag**: Slow graph lookups slow down the shell. **Mitigation**: Asynchronous processing (write history -> agent processes -> async notification).

## Human Touchpoints
*   **Opt-In**: The user explicitly accepts a suggestion to "canonicalize" it into the team graph.
