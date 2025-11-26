# Design 1: The Mapper (Conservative)

## Purpose
Legacy codebases are often terrifying to touch because no one understands the dependencies. "The Mapper" is a read-only agent that acts as a surveyor. It explores the codebase, builds a knowledge graph of entities (classes, functions, files), and generates human-readable documentation ("The Atlas"). It never modifies code.

## Loop Structure
1. **Scout:** Iterate through the file system.
2. **Analyze:** Parse files (using regex/grep) to identify symbols, imports, and function signatures.
3. **Record:** Store entities and relations in the MCP Memory graph.
   - `Entity: FunctionX` -> `Relation: Calls` -> `Entity: FunctionY`
4. **Synthesize:** Periodically generate markdown reports:
   - `dependency_map.md`: Who calls whom?
   - `dead_code_candidates.md`: Symbols with zero incoming edges.
   - `complexity_hotspots.md`: Files with high line counts or cyclomatic complexity (approximated by indentation/keywords).
5. **Sleep:** Wait for file system changes (poll or trigger) to update the graph.

## Tool Usage
- **filesystem:** `list_directory_recursive`, `read_text_file` to traverse the codebase.
- **grep:** `grep_search` to find usages of a symbol across the project to confirm dependencies.
- **memory:** `create_entities`, `create_relations` to build the persistent understanding of the system.
- **text-editor:** (Not used for code) Used to write the markdown reports.

## Memory Architecture
- **Nodes:** Files, Classes, Functions, Variables.
- **Edges:** Imports, Inherits, Calls, Reads, Writes.
- **Persistence:** The MCP Memory graph acts as the long-term database. If the agent restarts, it queries the graph before re-indexing files.

## Failure Modes
- **False Positives:** Regex-based parsing might misidentify comments as code.
  - *Recovery:* Human can flag nodes as "invalid" in the graph; Agent respects exclusion list.
- **Graph Explosion:** Too many nodes for the memory provider.
  - *Recovery:* Limit scope to specific directories or abstraction levels (only classes/public functions).

## Human Touchpoints
- **Configuration:** Human defines `root_dir` and `exclusion_patterns`.
- **Consumption:** Human reads the generated Markdown reports to make informed refactoring decisions.
