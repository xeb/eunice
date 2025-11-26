# Design 1: The Failure Librarian (Conservative)

## Purpose
To proactively identify unhandled exceptions and known failure modes in software dependencies by cross-referencing local code with aggregated web knowledge.

## Loop Structure
1.  **Dependency Scan:** Agent parses dependency files (e.g., `requirements.txt`, `package.json`) to identify used libraries.
2.  **Knowledge Retrieval:** For each library, the agent queries the Web (Brave Search) for:
    *   "common exceptions [library name]"
    *   "[library name] production failure modes"
    *   "[library name] best practices error handling"
3.  **Graph Construction:** Stores findings in the Memory Graph:
    *   `(Entity: Library) -- hasFailureMode --> (Entity: ExceptionType)`
    *   `(Entity: ExceptionType) -- requiresStrategy --> (Entity: MitigationStrategy)`
4.  **Code Audit:** Uses `grep` to locate usages of the library in the codebase.
5.  **Gap Analysis:** Checks if the identified `ExceptionType` is caught in the surrounding scope of the usage.
6.  **Reporting:** Generates a "Resilience Report" listing detected gaps (e.g., "Line 45 calls `boto3.client` but does not catch `ClientError`").

## Tool Usage
*   **filesystem:** Read dependency files and source code.
*   **web_brave_web_search:** Gather "Failure Knowledge" from StackOverflow, GitHub Issues, and blogs.
*   **memory:** Persist the relationship between libraries and their specific failure modes (so it doesn't need to re-search common libs like `requests`).
*   **grep:** Locate function calls and `try/catch` blocks.

## Memory Architecture
*   **Nodes:** `Library`, `Function`, `Exception`, `CodeLocation`.
*   **Edges:** `calls`, `raises`, `handles`, `vulnerableTo`.

## Failure Modes
*   **False Positives:** Flagging an error as unhandled when it propagates intentionally. (Mitigation: Allow user to mark "Intentional Propagation").
*   **Hallucinated Exceptions:** Web search finding deprecated or irrelevant exceptions. (Mitigation: Cross-reference with official docs).

## Human Touchpoints
*   **Report Review:** The user reads the generated Markdown report.
*   **Knowledge Validation:** User confirms if a specific "Failure Mode" is relevant to their context.
