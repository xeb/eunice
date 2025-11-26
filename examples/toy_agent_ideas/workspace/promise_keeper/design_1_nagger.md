# Design 1: The Promise Keeper (Conservative Variant - "The Nagger")

## Purpose
To visualize and track "Comment Insolvency" without modifying code. It acts as a "Credit Bureau" for technical debt, assigning a reliability score to developers based on how fast they resolve their TODOs.

## Loop Structure
1. **Scan**: Runs `grep` to find all `TODO`, `FIXME`, `HACK` comments.
2. **Blame**: Runs `git blame` on each line to identify the Author and Date.
3. **Ingest**: Updates the Memory Graph:
   - Nodes: `Developer`, `Promise` (the comment).
   - Edges: `MADE_BY`, `LOCATED_IN`.
   - Properties: `age_days`, `severity`.
4. **Report**: Generates a `DEBT_REPORT.md` file in the root:
   - "Top 10 Oldest Promises"
   - "Developer Credit Scores" (Ratio of Fixed vs. Stale TODOs).
   - "Inflation Rate" (New TODOs vs. Resolved TODOs this week).

## Tool Usage
- **grep**: `grep -r "TODO" .` to find artifacts.
- **shell**: `git blame` execution.
- **memory**: Stores the historical trend. Use `memory_add_observations` to track "This TODO existed last week".
- **filesystem**: Writes the read-only report.

## Memory Architecture
- **Entities**: `User`, `File`, `Promise`.
- **Relations**: `User PROMISED Promise`, `Promise IN_FILE File`.
- **Logic**: If a Promise ID (hash of content+location) disappears, it is marked "PAID". If it stays, "INTEREST" (age) accumulates.

## Failure Modes
- **False Positives**: Detects "TODO" in documentation or strings. (Mitigation: File extension filtering).
- **Renames**: Git blame might get confused by file moves. (Mitigation: Use `git blame -C`).
- **Shaming**: Low credit scores might demoralize the team. (Mitigation: Make scores private or opt-in).

## Human Touchpoints
- **Passive**: Humans only read the report. No active prompts.
