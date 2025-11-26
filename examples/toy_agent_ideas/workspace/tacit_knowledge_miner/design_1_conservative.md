# Design 1: The Bus Factor Auditor (Conservative)

## Purpose
To quantitatively measure and report on "Knowledge Risk" within a codebase. It identifies "hero" developers whose departure would imperil the project and highlights "Dark Zones" of code that are complex, critical, and understood by few.

## Loop Structure
**Trigger:** Scheduled (Weekly) or CI/CD Pipeline (Post-Merge).

1.  **Repository Scan:**
    *   Walk the file tree using `filesystem`.
    *   For each file, execute `shell` commands (`git blame`, `git log`) to calculate ownership percentages.
    *   Calculate "Code Complexity" using `grep` (counting indentations, conditionals, or length) or a linter if available.
2.  **Risk Calculation:**
    *   Combine Ownership + Complexity + Recency.
    *   *Formula:* Risk = (Complexity * (1 / Bus_Factor)) * Criticality_Weight.
3.  **Report Generation:**
    *   Generate a Markdown report in `docs/audit/knowledge_risk_report_[date].md`.
    *   List top 10 "At Risk" files.
    *   List top 5 "Key Person Dependencies".
4.  **Notification:**
    *   (Optional) Post a summary to a designated channel or PR.

## Tool Usage
*   **shell:** Executing `git` commands (`git shortlog -sn`, `git blame --line-porcelain`).
*   **filesystem:** listing files, writing reports.
*   **grep:** Simple complexity heuristics (e.g., counting `if`, `for`, `while`, nested braces).

## Memory Architecture
*   **Stateless/Filesystem:** This design relies on the Git history as the source of truth. It does not maintain an internal database. It outputs static artifacts (Markdown reports) that serve as the memory of the audit.

## Failure Modes
*   **Git History Pollution:** Large refactors or reformatting (Prettier/ESLint) skewing blame data. *Recovery:* Allow a `.git-ignore-revs` configuration to skip specific commits.
*   **False Positives:** High complexity file that is actually dead code. *Recovery:* Cross-reference with import/usage analysis (grep).

## Human Touchpoints
*   **Review:** Humans read the generated report.
*   **Configuration:** Humans tune the "Criticality" of specific directories (e.g., `src/core` vs `src/tests`).
