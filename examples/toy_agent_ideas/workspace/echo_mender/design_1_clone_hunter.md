# Design 1: The Clone Hunter (Conservative)

## Purpose
To prevent "incomplete fix" bugs where a developer patches one instance of a defect but misses identical copy-pasted versions elsewhere in the codebase.

## Loop Structure
1.  **Monitor:** Polls `git log` for new commits.
2.  **Analyze:** When a commit is detected, it isolates the "removed" lines from the diff.
3.  **Search:** It cleans these lines (removing whitespace/context) and runs exact-match and fuzzy-match `grep` searches across the entire project.
4.  **Report:** If matches are found (meaning the buggy code still exists elsewhere), it generates a report in `workspace/reports/potential_clones_<timestamp>.md`.
5.  **Notify:** (Optional) Can echo a warning to the shell or updated a status file.

## Tool Usage
*   `shell`: To run `git diff`, `git log`, and `grep`.
*   `filesystem`: To read files and write reports.
*   `grep`: Used for fast literal string searching of the "buggy" code snippets.

## Memory Architecture
*   **Stateless:** This variant is purely reactive to the current commit. It does not maintain a long-term graph. It relies on the filesystem for reports.

## Failure Modes
*   **False Positives:** The "removed code" might be generic (e.g., a closing brace or common variable declaration).
*   **Mitigation:** The agent filters out lines shorter than N characters or containing only common keywords.
*   **Missed Matches:** Slight whitespace variations or variable renaming will defeat the exact match.

## Human Touchpoints
*   **Passive:** The human reads the generated report. The agent does **not** modify code.
