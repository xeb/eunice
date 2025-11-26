# Design 2: The Evolutionary Historian

## Purpose
An experimental, fully autonomous agent that not only records history but *organizes* it. It mimics the way human historians retroactively classify eras (e.g., "The Bronze Age"). It evolves its own understanding of the domain.

## Loop Structure
1. **Continuous Ingestion:** Runs every hour.
2. **Graph Analysis (The Critic):**
   - Every 24 hours, the agent pauses ingestion to analyze the graph topology.
   - **Clustering:** Detects dense clusters of nodes.
   - **Concept Emergence:** If many nodes share a new keyword, it promotes that keyword to a first-class Entity Type.
3. **Graph Refactoring:**
   - **Splitting:** If a node "AI" has 10,000 connections, it splits it into "Generative AI", "Symbolic AI", etc., based on neighbor context.
   - **Pruning:** Removes "leaf" nodes that haven't been referenced in >30 days (moves them to "Cold Storage" files).
4. **Synthesis:** Writes a "State of the Union" essay explaining *why* it changed the graph structure.

## Tool Usage
- **memory:** Heavy use of `memory_read_graph` and `memory_delete_*` for refactoring.
- **grep:** Scans its own logs to find recurring unmapped terms.
- **web:** Search for definitions of emerging terms.

## Memory Architecture
- **Dynamic Schema:** The agent can create new Entity Types and Relation Types on the fly.
- **Temporal Versioning:** Relations have `valid_from` and `valid_to` properties.
- **Meta-Memory:** Stores "Decisions" as entities in the graph (e.g., "Decision: Split Node X" -> caused_by -> "Cluster Density").

## Failure Modes
- **Concept Drift:** It might aggressively split concepts until they are meaningless.
- **Looping:** It might continuously merge and split the same nodes.
- **Recovery:** A "Rollback" feature that restores the graph from a previous night's snapshot using filesystem backups.

## Human Touchpoints
- **Observer:** Humans can watch the "State of the Union" reports but ideally do not intervene.
- **Emergency Stop:** If the graph size explodes or the agent gets stuck in a loop.
