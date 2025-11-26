# Design 1: The Compliance Sentinel

## Purpose
A conservative, high-reliability agent focused on **Static Analysis and Reporting**. It acts as a specialized linter that doesn't just check syntax but correlates code patterns with WCAG violations, generating detailed compliance reports without modifying code automatically.

## Loop Structure
1.  **Scan Phase:** Periodically (or on git hook) scans the codebase using `filesystem` and `grep` to identify modified UI components (HTML, JSX, Vue, Svelte).
2.  **Audit Phase:** Checks these components against a local library of accessibility regex patterns (e.g., `img` without `alt`, `div` with `onClick` but no `role`).
3.  **Validation Phase:** Uses `web` tool to cross-reference potential violations with the latest WCAG guidelines (fetching specific success criteria).
4.  **Reporting Phase:** Generates a Markdown report in `accessibility_reports/` with severity levels, code snippets, and links to official documentation.
5.  **Notification Phase:** Updates a "Compliance Status" entity in `memory`.

## Tool Usage
*   **grep:** Primary engine for finding accessibility anti-patterns (e.g., `tabIndex > 0`, non-semantic tags).
*   **filesystem:** Reads file content to provide context around matches.
*   **web:** Fetches live WCAG definitions to ensure reports cite current standards.
*   **memory:** Stores the "Ignore List" (false positives flagged by humans).

## Memory Architecture
*   **Entities:** `Component`, `Violation`, `Standard` (WCAG Criterion).
*   **Relations:** `Component HAS Violation`, `Violation VIOLATES Standard`.
*   **Purpose:** Tracks the "Health" of the codebase over time. Allows the agent to say "This component has improved since last week."

## Failure Modes
*   **False Positives:** Regex is brittle. It might flag valid code.
    *   *Recovery:* Human adds the specific line/file to an "Ignore" list in Memory.
*   **Missed Context:** Cannot see runtime state (e.g., if a modal is actually visible).
    *   *Mitigation:* Explicitly labels issues as "Potential" vs "Definite".

## Human Touchpoints
*   **Review:** Humans read the generated reports.
*   **Configuration:** Humans update the `rules.yaml` or equivalent via the agent's memory (telling it to ignore certain paths).
