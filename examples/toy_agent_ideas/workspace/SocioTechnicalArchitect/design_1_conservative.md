# Design 1: The Silo Sentinel (Conservative)

## Purpose
A specialized linter that prevents "Socio-Technical Mismatches" by analyzing code coupling against contributor history. It flags pull requests that introduce dependencies between "Socially Disconnected" parts of the codebase, which typically lead to bugs.

## Loop Structure
1.  **Trigger:** Scheduled daily run or pre-commit hook.
2.  **Analysis:**
    *   Run `git log` to build a "Knowledge Map" (Who touches what?).
    *   Run `grep` to build a "Dependency Map" (What imports what?).
3.  **Evaluation:**
    *   For every dependency `A -> B`:
        *   Calculate `AuthorOverlap(A, B)`.
        *   If `Overlap < Threshold` (e.g., 0 common authors), flag as "High Risk Crossing".
4.  **Reporting:**
    *   Generate `conway_report.md` listing risky dependencies.
    *   Optional: Comment on PR (via CLI tool if configured).

## Tool Usage
*   **shell:** `git log --pretty=format:...`, `git diff`
*   **grep:** Finding `import`, `require`, `include`.
*   **filesystem:** Reading code, writing reports.

## Memory Architecture
*   **Stateless/Low-State:** Recomputes metrics on the fly or caches them in a simple JSON file (`.conway_cache.json`) to avoid re-parsing the whole git history.

## Failure Modes
*   **Git History Noise:** Large refactors or formatting changes skew ownership. (Mitigation: Ignore commits with >100 file changes).
*   **Language Parsing:** Grep is imprecise for dependencies. (Mitigation: Support distinct "Parser Plugins" for TS, Py, Go).

## Human Touchpoints
*   **Configuration:** Users define "Team" aliases (e.g., "Alice + Bob = Frontend Team").
*   **Triage:** Humans review the report and decide to "Allow" (whitelist) or "Refactor".
