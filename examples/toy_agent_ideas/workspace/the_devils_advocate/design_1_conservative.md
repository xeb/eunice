# Design 1: The Critical Reviewer (Conservative)

## Purpose
A "smart linter" for high-level architectural and product decisions. Instead of checking syntax, it checks for "Common Failure Patterns" in design documents, pull requests, and readme files. It ensures that critical non-functional requirements (NFRs) like "observability", "rollback", and "compliance" are explicitly addressed.

## Core Loop
1.  **Trigger:** User asks for a review of a specific file (doc or code) or the agent runs on a schedule (daily scan of `docs/`).
2.  **Analysis:** The agent reads the target content.
3.  **Retrieval:** It queries its `memory` for a list of active "Checklists" (e.g., "Security Best Practices", "Scalability Checklist", "UX Anti-Patterns").
4.  **Verification:** For each checklist item, it performs a targeted `grep` or `web_search` to verify if the concern is addressed.
    *   *Example:* Item "Rate Limiting". Action: Grep for `rate_limit`, `throttle`, or 429 handling.
    *   *Example:* Item "Competitor Feature Parity". Action: Web search "Feature X in Competitor Y".
5.  **Report:** It generates a report in `reviews/` highlighting missing considerations.

## Tool Usage
*   **filesystem:** Read `README.md`, `DESIGN.md`, and source code.
*   **memory:** Store the "Checklists" and "Known false positives" (suppressions).
*   **grep:** Search for keywords proving a requirement is met.
*   **web:** Look up current best practices (e.g., "OWASP Top 10 2025") to update checklists.

## Memory Architecture
*   **Nodes:** `Checklist`, `ChecklistItem`, `ReviewTarget` (File), `Finding`.
*   **Relations:** `Checklist HAS Item`, `File SATISFIES Item`, `File VIOLATES Item`.
*   **Persistence:** The graph stores the *compliance status* of the project over time.

## Failure Modes
*   **False Positives:** Naive keyword matching might think "Rate limiting" is handled because the word exists in a comment "TODO: Add rate limiting".
    *   *Recovery:* User adds the specific finding ID to a "Suppression" list in Memory.
*   **Stale Checklists:** Best practices change.
    *   *Recovery:* Monthly "Self-Update" mode where it searches the web for new checklists.

## Human Touchpoints
*   **Setup:** User defines which checklists matter.
*   **Review:** User reads the report and marks items as "Won't Fix" or "False Positive".
