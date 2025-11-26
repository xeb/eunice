# Design 3: The Contract Negotiator (Hybrid)

## Purpose
To treat external library calls as "Legal Contracts" that need to be explicitly documented and negotiated in the code itself using rich annotations.

## Loop Structure
1.  **Interface Discovery:** Identify all "Exit Points" in the system (calls to external libs, APIs, OS).
2.  **Contract Research:**
    *   **Official Terms:** Fetch official docs to find stated exceptions.
    *   **Common Law:** Search forums for "undocumented" behaviors (e.g., "API returns 200 OK on error").
3.  **Annotation Injection:** The agent edits the code to add Javadoc/Docstring style annotations above the call site:
    ```python
    # @contract:risk High
    # @contract:failure_mode ConnectionResetError (Network flake)
    # @contract:failure_mode 503 Service Unavailable (Upstream maintenance)
    # @contract:undocumented Returns 200 with body "error" sometimes
    response = requests.get(url)
    ```
4.  **IDE Integration:** These comments serve as warnings to the developer.
5.  **Runtime Verification:** A wrapper (aspect-oriented) reads these tags and enforces the contract (logging a violation if an undocumented error occurs).

## Tool Usage
*   **text-editor:** Inserting rich comments/annotations.
*   **web_brave_web_search:** Researching the "Contract".
*   **filesystem:** Reading code.

## Memory Architecture
*   **Knowledge Graph:** Stores the "Contract Definitions" for libraries so they can be reused across projects.

## Failure Modes
*   **Code Clutter:** Adding too many comments makes code unreadable.
*   **Stale Contracts:** The web research might be outdated compared to the library version used.

## Human Touchpoints
*   **Contract Review:** Developer accepts/rejects the proposed annotations.
