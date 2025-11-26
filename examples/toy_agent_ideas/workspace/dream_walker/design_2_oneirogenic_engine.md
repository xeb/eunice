# Design 2: The Oneirogenic Engine (Innovative)

## Purpose
To generate *novel* and *unexpected* solutions by algorithmically forcing "bisociation" (connecting unrelated frames of reference) during a simulated "REM sleep" cycle.

## Loop Structure
1.  **Waking (Data Ingestion)**:
    *   Agent monitors `project/` folder changes.
    *   Builds a "Context Graph" in Memory (Entities: Functions, Concepts, TODOs).
2.  **Sleep Onset (Hypnagogia)**:
    *   User marks a node in the graph as "The Block".
    *   Agent disconnects from live web search (to avoid local maxima).
3.  **REM Cycle (The Dream)**:
    *   **Random Walk**: Agent picks a random, distant node in its Knowledge Graph (or a random Concept from a pre-loaded "Universal Ontology").
    *   **Forced Connection**: It uses an LLM (simulated via text processing or creative prompting) to ask: "How is [The Block] like [Random Concept]?"
    *   **Mutation**: It generates a metaphorical "solution" based on this analogy.
    *   *Example*: "How is 'Database Locking' like 'Traffic Roundabouts'?" -> "Proposal: Implement yield-based locking."
4.  **Waking (Consolidation)**:
    *   Agent filters "Dreams" for basic logical coherence.
    *   Writes the top 3 wildest metaphors to `dream_journal.md`.

## Tool Usage
*   **memory**: heavy usage. The "Brain" of the agent. Stores concepts, metaphors, and the problem graph.
*   **web**: Used *before* sleep to populate the "Universal Ontology" (fetching random Wikipedia pages, art history, biology).
*   **filesystem**: Only for journaling.

## Memory Architecture
*   **Graph**: Dense, highly interconnected.
    *   Entities: `Concept`, `Metaphor`, `Blocker`.
    *   Relations: `is_analogous_to`, `inspires`, `contradicts`.
*   **Dynamics**: Edges decay over time if not reinforced (forgetting).

## Failure Modes
*   **Hallucination**: The analogies are nonsensical. (Feature, not bug? User filters them).
*   **Graph Bloat**: Too many random concepts. (Mitigation: Aggressive "synaptic pruning" of unused nodes).

## Human Touchpoints
*   **Interpretation**: The user must interpret the abstract metaphors. The agent provides the spark, not the code.
