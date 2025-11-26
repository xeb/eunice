# Design 2: The Graph-Based Team Mapper (Innovative)

## Purpose
An autonomous researcher that discovers the *actual* organizational structure (Hidden Teams) by observing digital traces, effectively reverse-engineering the Org Chart from the bottom up and contrasting it with the official structure.

## Loop Structure
1.  **Ingestion (Continuous):**
    *   Watches `git` history, Slack exports (if provided), and Issue trackers.
2.  **Graph Construction (Memory):**
    *   **Nodes:** `Person`, `File`, `Module`, `Concept`.
    *   **Edges:**
        *   `Person --(edited)--> File`
        *   `File --(imports)--> File`
        *   `Person --(commented_on)--> Issue`
        *   `Person --(reviewed)--> Person`
3.  **Topology Analysis:**
    *   Detects **"Congruence Gaps"**: Where `File A` depends on `File B`, but `Team(File A)` has no edges to `Team(File B)`.
    *   Detects **"Knowledge Islands"**: Modules owned by a single person with no backup (Bus Factor = 1).
4.  **Visualization:**
    *   Generates a Graphviz/Mermaid file visualizing the "Real" org chart.

## Tool Usage
*   **memory:** Storing the complex graph of social and technical relations.
*   **shell:** `git`, `gh` (GitHub CLI) for metadata.
*   **web:** Fetching public org data (optional).

## Memory Architecture
*   **Graph-Native:** Uses the Memory MCP to store a persistent, evolving model of the organization.
*   **Time-Aware:** Edges have timestamps to track "Drift" (e.g., "Alice used to work on Backend, now works on Frontend").

## Failure Modes
*   **Privacy Concerns:** analyzing user behavior might be invasive. (Mitigation: Anonymize names by default, strictly local operation).
*   **False Positives:** "Drive-by" commits (typo fixes) implying strong relationships. (Mitigation: Weighted edges based on lines changed).

## Human Touchpoints
*   **Insight Querying:** User asks "Who should I talk to about `auth.ts`?" Agent answers based on the graph.
*   **Alerts:** Agent proactively warns "You are creating a dependency on `Billing` module, but you have no relationship with `Billing Team`. Expect delays."
