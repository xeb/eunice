# Design 3: The Migration Assistant (The "Sherpa")

## Purpose
To assist developers in long-running, painful migrations (e.g., "Angular to React", "Python 2 to 3") by managing the "Migration Debt" ledger and autonomously chipping away at simple conversions in the background.

## Loop Structure
1. **Initialize:** User defines a "Migration Goal" (e.g., "Replace `request` with `axios`").
2. **Audit:** Agent greps the codebase to build a "Migration Backlog" (list of files containing `request`).
3. **Strategize:** Agent groups files by complexity (LOC, import count).
4. **Execute (Background):**
   - Picks the simplest file.
   - Creates shadow branch.
   - Attempts substitution (using Regex or Web-sourced examples).
   - Runs tests *scoped to that file*.
5. **Review:**
   - If tests pass, pushes a Draft PR titled "chore: Migrate file X to axios".
   - If tests fail, adds specific error context to the Backlog entry.
6. **Learn:**
   - If user fixes a failed PR, Agent analyzes the diff to improve its future substitution logic.

## Tool Usage
- **memory:** Tracking the "Backlog" state (To Do, In Progress, Blocked, Done).
- **text-editor:** Making granular edits.
- **shell:** Running scoped tests (e.g., `npm test -- test/file_spec.js`).
- **web:** Looking up "Equivalent of X in library Y".

## Memory Architecture
- **Entity:** `MigrationTask` (File Path, Complexity Score).
- **Relation:** `MigrationTask` HAS_STATUS `Blocked`.
- **Observation:** "File X uses deprecated method .pipe() which has no direct equivalent in Axios."

## Failure Modes
- **Subtle Bugs:** Regex replacements missing nuance. *Fix:* Rely heavily on compilation/linting errors to catch syntax issues.
- **Merge Conflicts:** Shadow branches getting out of date. *Fix:* Rebase strategy before every attempt.

## Human Touchpoints
- **Definition:** User sets the initial migration rule/goal.
- **Approval:** User reviews and merges the micro-PRs.
