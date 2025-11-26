# Final Design: The EcosystemMapper

## Synthesis
The **EcosystemMapper** combines the autonomy of the *Recursive Explorer* with the observable state of the *Periodic Crawler*. It avoids the bottleneck of the *Hybrid Curator* but adds a "Confidence Score" to the memory graph to allow for eventual consistency and post-hoc human cleanup.

## Core Insight
Instead of asking for permission *before* writing (Design 3), the agent writes *everything* to the Memory Graph but tags unverified data with `confidence: low`. It then generates a "Weekly Digest" file that highlights low-confidence nodes for the user to either "Confirm" (boost confidence) or "Prune" (delete).

## Architecture

### 1. The Discovery Loop (Autonomous)
- **Frequency**: Continuous background process (or frequent cron).
- **Action**:
  1. Pick a `Topic` node with `status: unexplored`.
  2. **Search**: `web_brave_web_search(query=topic)`.
  3. **Extract**: Parse top results. Identify related tools/libraries.
  4. **Commit**:
     - Call `memory_create_entities` for new items.
     - **Crucially**: Add property `confidence: 0.5` and `source: <url>`.
     - Call `memory_create_relations`.
  5. **Update**: Set current node `status: explored`.

### 2. The Pruning Loop (Maintenance)
- **Frequency**: Runs after every 50 new nodes or daily.
- **Action**:
  1. Scan graph for dense clusters (using `memory_search_nodes` or graph traversal).
  2. Identify "orphan" nodes (low connectivity) created > 1 week ago.
  3. Delete orphans automatically (garbage collection).

### 3. The Reporting Interface (Human)
- **Action**:
  - Generate `workspace/EcosystemMapper/weekly_report.md`.
  - List "Top New Emerging Entities" (high connectivity, recent creation).
  - List "Controversial/Low-Confidence Entities" for review.

## MCP Toolset
1. **memory**: Primary storage. Uses `observations` to store raw text snippets supporting the relationship.
2. **web**: Brave Search for discovery.
3. **fetch**: For verifying existence of GitHub repos or documentation pages.
4. **filesystem**: For logging and delivering the Markdown reports.

## Failure & Recovery
- **Runaway Graph**: If the graph grows too fast, the agent checks `filesystem_get_file_info` on its own logs. If > 100MB, it pauses and waits for human reset.
- **Poisoned Data**: If a search result is SEO spam, the agent might ingest garbage.
  - *Mitigation*: The "Confidence Score" system. Nodes usually need > 1 source to be promoted to high confidence.

## Impact
This agent allows a developer to say "Map the `Rust Async` ecosystem" and come back a week later to a rich, browsable knowledge graph and a high-level summary of the most important players, without having to manually curate every entry.
