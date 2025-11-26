# Design 3: The Refactoring Historian (Hybrid)

## Purpose
To audit the codebase against its own history, identifying "Zombie Code" that missed previous refactorings or re-introduced fixed bugs.

## Loop Structure
1.  **Mining:** Periodically scans the entire `git log` history (not just recent commits).
2.  **Indexing:** Builds a database of "Refactoring Events" (commits touching >5 files or changing method signatures).
3.  **Audit:** For each Refactoring Event, it checks the *current* HEAD of the repo.
    *   Does the old method signature exist?
    *   Does the old "bad" logic exist?
4.  **Blame:** If it finds the old pattern, it uses `git blame` to see *when* it was re-introduced.
5.  **Educate:** It creates a "Historical Context" report for the developer who introduced the regression, explaining *why* that pattern was removed years ago.

## Tool Usage
*   `filesystem`: Deep reading of current files.
*   `shell`: Heavy use of `git log -p`, `git blame`, `git rev-list`.
*   `memory`: Stores the "Refactoring Timeline" and "Author Reliability Scores".
*   `text-editor`: Reads file contents for verification.

## Memory Architecture
*   **Temporal Graph:** Maps code entities to their "Death Dates".
    *   *Observation:* "Method `oldAuth` was deprecated in commit X on date Y."
    *   *Check:* "Is `oldAuth` present in HEAD?"

## Failure Modes
*   **False Alarms:** Sometimes code is reverted intentionally.
*   **Performance:** Scanning full git history is expensive.
*   **Mitigation:** Incremental indexing and caching results in `memory`.

## Human Touchpoints
*   **Educational:** The output is a learning tool ("Did you know this was fixed in 2023?").
