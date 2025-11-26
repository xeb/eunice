# Design 1: The Time-Sheet Auditor

## Purpose
To provide a rigorous, evidence-based "Post-Mortem" for personal time estimation by comparing explicit user logs against irrefutable git timestamps.

## Loop Structure
1. **Trigger:** Runs nightly or on-demand via CLI.
2. **Input:** Reads a user-maintained `timesheet.md` or `tasks.json` where the user records "Task Name" and "Estimated Duration".
3. **Verification:**
   - Scans `git log` for commits containing the "Task Name" or ID.
   - Calculates the time delta between the first and last commit for that task ID.
   - Adds "Session Gaps" (if no commits for >4 hours, assume break).
4. **Analysis:** Compares User Estimate vs. Git Reality.
5. **Output:** Appends a report to `audit_log.md`: "Task X: Est 2h, Actual 6h (3.0x Optimism Factor)".

## Tool Usage
- **filesystem:** Read `timesheet.md`, write `audit_log.md`.
- **shell:** `git log --grep="TASK-123" --format="%ct"` to get timestamps.
- **grep:** To parse the markdown structure.

## Memory Architecture
- **Stateless:** This design is simple and file-based. It does not maintain a persistent graph, relying instead on the historical log file.

## Failure Modes
- **Commit Discipline:** If the user forgets to put the Task ID in the commit message, the agent sees 0h actual time.
- **Squash Commits:** Squashing destroys the timestamp history, breaking the duration calculation.

## Human Touchpoints
- User must manually maintain the `timesheet.md`.
- User must use specific Commit Message conventions (e.g., "Feat: [TASK-123] ...").
