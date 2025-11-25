# Design 2: The Graph Storyteller (Innovative)

## Purpose
The **Graph Storyteller** treats news not as a list of articles, but as a growing graph of **Events**, **Actors**, and **Causal Links**. It aims to construct a persistent, queryable history of a narrative, allowing users to ask "How did we get here?" and visualising the evolution of a complex situation (e.g., a geopolitical crisis or a legal trial).

## Loop Structure
1.  **Continuous Monitoring:** Runs in a loop, polling news sources for defined "Narrative Arcs".
2.  **Extraction & Ingestion:**
    *   For each article, extracts Entities (People, Orgs) and Events (Actions with timestamps).
    *   Uses `memory_search_nodes` to find existing entities.
    *   Uses `memory_create_entities` and `memory_create_relations` to graft new nodes onto the graph.
    *   *Key Relation Types:* `CAUSED`, `FOLLOWED_BY`, `CONTRADICTS`, `PARTICIPATED_IN`.
3.  **Narrative Analysis:**
    *   Traverses the graph to identify "active branches" (recent chains of events).
    *   Detects "Narrative Shifts" (e.g., a node's sentiment polarity flips, or a new causal cluster emerges).
4.  **Synthesis:**
    *   Generates a "Story So Far" narrative by walking the graph from the root event to the leaf nodes.
    *   Produces a visualizable graph definition (e.g., Mermaid or DOT format).

## Tool Usage
*   **memory:** HEAVY usage. This is the core of the agent. The graph *is* the story.
    *   `memory_create_entities`, `memory_create_relations`, `memory_read_graph`.
*   **web:** `web_brave_news_search` for raw material.
*   **fetch:** `fetch_fetch` to retrieve full article text for deep relation extraction.

## Memory Architecture
*   **Event-Centric Graph:**
    *   Nodes: `Event`, `Person`, `Organization`, `Location`, `Claim`.
    *   Edges: `PRECEDES`, `CAUSES`, `ATTRIBUTED_TO`, `DENIES`.
*   **Temporal Indexing:** Time is a first-class property of Event nodes, allowing the agent to "replay" the graph state at any past date.

## Failure Modes
*   **Graph Explosion:** Too many minor nodes (noise) make the graph traversing expensive.
    *   *Mitigation:* Strict entity importance filtering (Pruning) and "Cluster nodes" for minor events.
*   **Conflicting Truths:** Different sources report contradictory facts.
    *   *Mitigation:* The graph supports `Claim` nodes linked to specific `Source` entities, allowing the graph to hold multiple conflicting versions of reality simultaneously.

## Human Touchpoints
*   **Graph Pruning:** Humans might need to merge duplicate nodes or delete irrelevant branches.
*   **Narrative Querying:** The primary interface is the user asking questions about the graph.
