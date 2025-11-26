# Design 1: The Runbook Scribe

## Purpose
To automatically document manual incident response sessions into clean, reproducible Markdown runbooks. It solves the "forgotten fix" problem where an engineer solves a complex issue in the terminal but forgets to document the steps.

## Loop Structure
1. **Passive Monitoring**: The agent periodically scans (tail) the shell history file (e.g., `.bash_history` or `.zsh_history`).
2. **Session Detection**: It identifies bursts of activity separated by significant pauses, classifying them as "Sessions."
3. **Context Enrichment**: For each command in a session, it runs `grep_search` on the codebase to see if the command relates to known scripts or config files.
4. **Draft Generation**: It creates a `drafts/session_<timestamp>.md` file.
5. **Human Review**: The user reviews the draft, adds comments, and moves it to `runbooks/approved/`.
6. **Ingestion**: The agent reads the approved runbook into the `memory` graph for future reference.

## Tool Usage
*   **shell**: `tail -n 50 ~/.bash_history` to get recent commands.
*   **filesystem**: Write Markdown drafts.
*   **grep**: Search for context (e.g., if user ran `./restart_api.sh`, grep that file to see what it does).
*   **memory**: Store associations between "Error Messages" (if provided by user context) and "Command Sequences".

## Memory Architecture
*   **Nodes**: `Session`, `Command`, `File`, `Topic`.
*   **Relations**: `Session -> executed -> Command`, `Command -> modified -> File`.
*   **Graph**: A historical log of "Who did what, when".

## Failure Modes
*   **Noise**: Captures `ls`, `cd`, and typos. Needs a filter to ignore trivial commands.
*   **Secrets**: Might capture API keys typed in the terminal. **Mitigation**: Regex scan for entropy/keys before writing to disk.

## Human Touchpoints
*   **Approval**: User must manually move/rename the draft to confirm it is a valid runbook.
