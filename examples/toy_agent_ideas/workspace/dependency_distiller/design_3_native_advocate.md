# Design 3: The Native Advocate (Educational/Evolutionary)

## Purpose
The "Native Advocate" focuses on **modernization**. It assumes that many dependencies exist to bridge gaps in older runtimes (browsers/node versions) that are now closed. It scans code for patterns that can be replaced by native platform features (e.g., replacing `axios` with `fetch`, `moment` with `Intl`, `lodash` with `ES6+`).

## Loop Structure
1.  **Knowledge Sync:**
    *   Use `web` to search for "You Might Not Need X" resources.
    *   Update an internal `memory` graph of `Library -> NativeReplacement`.
2.  **Pattern Match:**
    *   Scan the codebase for usages of known "obsolete" libraries.
3.  **Refactor Proposal:**
    *   For each match, generate a specific refactoring example showing the "Before" (Library) and "After" (Native) code.
    *   It does *not* automatically apply changes, as native replacements often have subtle API differences.
4.  **Education:**
    *   Insert comments or generate a "Migration Guide" markdown file explaining *why* the change is recommended (performance, security, bundle size).

## Tool Usage
*   **memory:** Stores a graph of `(Library Node) --[can_be_replaced_by]--> (Native Feature Node)`.
*   **web:** Continuously researches new platform features (e.g., "New JavaScript Set methods 2025").
*   **grep:** Finds usages.
*   **text-editor:** Inserts "deprecation/suggestion" comments directly into the code (optional mode).

## Memory Architecture
*   **Persistent Graph:** Builds a long-term database of "Modernization Rules" that grows as the agent researches more libraries.

## Failure Modes
*   **Browser Support:** Native features might not be supported in the user's target environment (e.g., IE11). The agent needs to check `browserslist`.
*   **Edge Cases:** Native implementations often throw errors differently than libraries.

## Human Touchpoints
*   **Review:** The user treats the agent as a "Senior Dev" doing a code review, deciding which suggestions to accept.
