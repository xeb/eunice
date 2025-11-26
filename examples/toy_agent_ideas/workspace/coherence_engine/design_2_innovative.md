# Design 2: The Truth Maintenance System (Innovative)

## Purpose
To create a persistent "Knowledge Graph" of the codebase that tracks claims made by different artifacts (docs, tests, code) and proactively identifies semantic contradictions.

## Core Philosophy
"Every line of documentation is a claim; every unit test is a proof." The agent reifies implicit knowledge into explicit graph nodes to find "Semantic Drift."

## Loop Structure
1.  **Ingestion**:
    *   Watches for file changes.
    *   On change, parses the file to extract "Fact Atoms".
    *   Example: Comment "Returns true if valid" -> `Claim(Source: File:Line, Subject: ReturnValue, Predicate: is, Object: TrueIfValid)`.
    *   Example: Code `return false;` -> `Reality(Source: File:Line, Subject: ReturnValue, Predicate: is, Object: False)`.
2.  **Graph Update (Memory)**:
    *   Updates the `memory` graph with new nodes/edges.
    *   Runs graph queries to find conflicting edges (e.g., Claim != Reality).
3.  **Validation**:
    *   If a Claim refers to an external fact (e.g., "See RFC 1234"), uses `web_brave_search` to verify the RFC exists and its title matches.
4.  **Actuation**:
    *   If a contradiction is found, it uses `text-editor` to inject a "Coherence Alert" comment directly above the discrepancy.
    *   Example: `// [!WARNING] Coherence Engine detected contradiction: Doc says X, Code implements Y.`

## Tool Usage
*   **memory**: Stores the graph of Entities (Classes, Functions, Docs) and Relations (DOCUMENTS, IMPLEMENTS, TESTS, CONTRADICTS).
*   **grep**: Rapidly finds usages to build "Usage" edges.
*   **web**: Verifies external references in comments.
*   **text-editor**: Injects alerts into the code.

## Memory Architecture
*   **Graph Database**: Uses the Memory MCP to store a rich semantic graph.
*   **Nodes**: `Artifact`, `Claim`, `Test`, `ExternalResource`.
*   **Edges**: `verifies`, `contradicts`, `references`, `implements`.

## Failure Modes
*   **Graph Bloat**: The memory graph could become too large. Strategy: Prune nodes not touched in 30 days.
*   **Misinterpretation**: NLP parsing of comments is error-prone. Strategy: Only flag high-confidence contradictions or use a "Confidence Score" property on edges.

## Human Touchpoints
*   **Alert Resolution**: Developer sees the injected comment and fixes the code or docs.
*   **Feedback**: Developer can delete the injected comment to acknowledge the fix (Agent detects deletion and updates graph).
