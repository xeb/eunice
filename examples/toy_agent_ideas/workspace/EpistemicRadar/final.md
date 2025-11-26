# Final Design: The Epistemic Radar

## Synthesis
The Epistemic Radar combines the **autonomy** of the Hunter-Gatherer with the **rigor** of the Council. It uses a graph-based persistence layer to track the state of a domain but relies on an internal "dialectical" process to ensure data quality. It is designed to run as a background daemon, slowly building a high-fidelity map of a changing landscape.

## Core Architecture

### 1. The Knowledge Graph (Memory)
The central source of truth.
- **Entities**: Technologies, Companies, People, Concepts.
- **Relations**: `COMPETES_WITH`, `BUILT_ON`, `CREATED_BY`, `DEPRECATED_BY`.
- **Properties**: `confidence_score` (0.0-1.0), `last_verified` (timestamp), `debate_file` (path to filesystem).

### 2. The Loop (The "Radar Sweep")
The agent operates in 30-minute "sweeps".

1.  **Scan**: Selects a node from the graph based on a priority queue (prioritizing old `last_verified` dates or high `centrality`).
2.  **Ping**: Uses `web_brave_web_search` to find recent news/docs about this entity.
3.  **The Filter (Simplified Council)**:
    - Generates a "Claim": "Entity X has released version Y."
    - Verification: Searches specifically to verify this claim.
    - If verified -> Update Graph.
    - If ambiguous -> Create a "Mystery" markdown file in `workspace/EpistemicRadar/mysteries/`.
4.  **The Expansion**:
    - Looks at the search results for *new* entities mentioned in high proximity to the target.
    - If a new entity appears significantly, it is added as a "Provisional" node (low confidence).
5.  **Pruning**:
    - Every 10th sweep, it checks for "Provisional" nodes that haven't gained connections. It deletes them (garbage collection).

### 3. Tool Usage Strategy
- **memory**:
  - `memory_read_graph`: To determine what to scan next.
  - `memory_create_entities`: To register new findings.
  - `memory_add_observations`: To log specific evidence (URLs, snippets) backing a node.
- **web**:
  - `web_brave_web_search`: General discovery.
  - `web_brave_news_search`: For the "Ping" phase (temporal updates).
- **filesystem**:
  - `logs/`: Execution logs.
  - `mysteries/`: Complex topics requiring human look (or future advanced agent passes).
  - `snapshots/`: Weekly JSON dumps of the graph for backup.

### 4. Innovation: The "Surprise" Metric
The agent calculates a "Surprise Score" for every sweep.
- If a search reveals a relation that contradicts the existing graph (e.g., "X is now owned by Y" when it was thought to be independent), this is High Surprise.
- High Surprise events trigger a special `alert_YYYY-MM-DD.md` file generation, effectively "waking up" the user.

## Implementation Roadmap
1.  **Initialize**: `setup.py` creates the directory structure and seeds the memory with 5-10 starting nodes.
2.  **Run**: `radar.py` enters the main loop.
3.  **Monitor**: User checks `mysteries/` folder and `alert` files.

## Failure & Recovery
- **Tool Failures**: Exponential backoff on web search errors.
- **Memory Corruption**: The `snapshots/` folder allows restoring the graph to a previous state.
- **Stuck Loop**: The agent keeps a `history` list of the last 50 visited nodes to avoid cycling between the same two entities.
