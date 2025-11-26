# Design 2: The Recursive Explorer (Innovative)

## Purpose
To autonomously discover and map an unknown or emerging ecosystem by following relationships deeply, building a persistent knowledge graph without human guidance.

## Loop Structure
1. **Bootstrap**: Starts with a single "Seed Node" in the Memory Graph (e.g., "Model Context Protocol").
2. **Select**: Queries `memory_search_nodes` to find "Unexplored" nodes (entities mentioned but not yet visited).
3. **Explore**:
   - Performs `web_brave_web_search` for the entity.
   - Visits top 3 results using `fetch_fetch`.
4. **Synthesize**:
   - Analyzes text to find *new* related entities and relationship types (e.g., "is compatible with", "competitor of").
   - Uses `memory_create_entities` and `memory_create_relations` to update the graph.
   - Marks the current node as "Explored".
5. **Repeat**: Loops immediately to Step 2.

## Tool Usage
- **memory**: The core brain. Stores the graph, tracks visited status, holds metadata.
- **web**: Brave search to find information on new entities.
- **fetch**: Deep dive into documentation or repositories.

## Memory Architecture
- **Graph Database**: Fully leverages the Memory MCP.
- **Nodes**: Projects, Companies, People, Concepts.
- **Edges**: `DEPENDS_ON`, `CREATED_BY`, `COMPETES_WITH`.
- **Self-Organizing**: The graph structure emerges from the data found.

## Failure Modes
- **Rabbit Holes**: Getting stuck in irrelevant sub-graphs (e.g., parsing common utility libraries). Recovery: Limit search depth or node relevance score.
- **Hallucination**: Extracting incorrect relationships. Recovery: Cross-reference with multiple sources before committing to memory.
- **Infinite Loops**: bouncing between two closely related entities. Recovery: "Visited" flags in memory.

## Human Touchpoints
- **Initialization**: Providing the initial seed.
- **Observation**: The human watches the graph grow in real-time.
- **Intervention**: Can manually delete irrelevant nodes via `memory_delete_entities` to prune the search space.
