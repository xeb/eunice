# Design 2: The Semantic Bridge (Innovative)

## Purpose
To identify "Functional Duplication" where different code achieves the same goal (e.g., three different `formatDate` functions), and propose unified abstractions. It moves beyond exact matching to **Semantic Convergence**.

## Loop Structure
1.  **Scanning**: Iterates through code files (`.ts`, `.py`, `.rs`).
2.  **Extraction**: Uses `grep` and Regex to extract function signatures, names, and docstrings.
3.  **Concept Mapping**:
    *   Tokenizes function names (e.g., "get_user_data" -> "get", "user", "data").
    *   Stores these in **Memory** as `Concept` nodes.
    *   Links functions to Concepts.
4.  **Convergence Analysis**: Finds Concepts that have implementation nodes in >N projects.
5.  **Proposal Generation**:
    *   Selects the "cleanest" implementation (heuristic: shortest line count, most recent edit, or presence of tests).
    *   Writes a `SharedUtils` proposal file.

## Tool Usage
*   `grep`: For fast signature extraction.
*   `memory`: To build the "Ontology of Functions".
*   `filesystem`: To read code and write proposals.

## Memory Architecture
*   **Nodes**: `Function`, `Concept` (token), `ArgumentType`.
*   **Edges**: `IMPLEMENTS_CONCEPT`, `TAKES_ARGUMENT`, `RETURNS_TYPE`.
*   **Insight**: If Project A has `sendEmail(to, body)` and Project B has `emailSender(recipient, content)`, the agent links them via the `email` concept.

## Failure Modes
*   **Hallucinated Similarity**: Suggesting two functions are the same when they differ in subtle, critical logic.
*   **Recovery**: The agent requires a human to "Ratify" the merge. It generates a "Diff View" showing the differences between the candidates.

## Human Touchpoints
*   **Ratification**: User reviews the "Proposal" file.
*   **Execution**: If approved, the user runs a script to move the code to a shared lib (manual step).
