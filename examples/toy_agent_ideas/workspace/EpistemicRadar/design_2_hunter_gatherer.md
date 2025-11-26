# Design 2: The "Hunter-Gatherer" (Innovative)

## Purpose
A fully autonomous, self-directed exploration agent. It doesn't just track *what* you tell it; it follows threads of interest to discover *unknown unknowns*. It aims to map the "adjacent possible" of a technology domain.

## Loop Structure (Continuous Autonomous)
1. **Objective Setting**: Reads a high-level goal (e.g., "Map the ecosystem of MCP servers").
2. **Graph Analysis**:
   - Uses `memory_read_graph` to find "leaf nodes" (entities with few connections) or "hot spots" (entities with recent high activity).
   - Selects a "Target Entity" to investigate.
3. **Foraging (Search)**:
   - Generates dynamic queries based on the Target (e.g., "competitors of X", "integrations for X", "X vs Y").
   - Uses `web_brave_web_search` (and potentially `fetch_fetch` to scrape docs) to gather raw text.
4. **Synthesis & Expansion**:
   - Analyzes search results to extract *new* entities and relations.
   - **Dynamic Ontology**: If it finds a new category (e.g., "Vector Database"), it creates it. It is not bound by a fixed schema.
   - Uses `memory_create_entities` and `memory_create_relations` aggressively.
5. **Reflection (Self-Correction)**:
   - Periodically (every N cycles) runs a "Consistency Check".
   - Uses `memory_search_nodes` to find duplicates or contradictions.
   - Merges nodes if they seem identical (e.g., "Node.js" and "NodeJS").
6. **Drift Detection**:
   - Uses `filesystem_write_file` to update a "heat map" of its exploration path.
   - If it strays too far from the original topic (e.g., starts mapping "Cooking Recipes" because of a metaphor), it uses a semantic distance check to prune its queue.

## Tool Usage
- **memory**: Dynamic and expanding. Uses the graph to guide its own search strategy (Graph Traversal as Planning).
- **web**: Broad search, following links.
- **grep**: Used on its own "thought logs" to identify repetitive behaviors or loops.

## Memory Architecture
- **Associative Memory**: The graph *is* the queue. Unexplored edges are "todos".
- **Decay**: Entities that haven't been observed in X cycles have their "confidence" score lowered (simulated via observation metadata).

## Failure Modes
- **Rabbit Holes**: Getting stuck in an irrelevant sub-graph. ADDRESSED BY: "Boredom" metric—if information gain drops, switch branches.
- **Graph Pollution**: Adding low-quality junk data. ADDRESSED BY: A separate "Janitor" process (or mode) that prunes low-connectivity nodes.

## Human Touchpoints
- **Goal Injection**: Human can insert a new "high priority" node to redirect the agent's attention.
- **Taxonomy Review**: Human might need to clean up messy entity types occasionally.
