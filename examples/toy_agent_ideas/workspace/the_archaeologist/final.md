# Final Design: The Refactoring Archaeologist

## Concept
The Refactoring Archaeologist is a persistent background agent designed for long-term stewardship of legacy codebases. Unlike standard "coding assistants" that react to prompts, the Archaeologist proactively explores the code, mapping dependencies and identifying "fossils" (dead code, deprecated patterns, missing documentation).

It does not commit changes directly. Instead, it curates **"Dig Sites"**—structured directories containing analysis, risk assessments, and ready-to-apply patches. The developer reviews these sites and acts as the "Museum Director," approving or rejecting the findings.

## Core Toolset
- **grep:** For fast, regex-based pattern matching and symbol discovery.
- **filesystem:** To traverse the "ruins" (directories) and write reports.
- **memory:** To build a persistent Knowledge Graph of the code structure, preventing re-analysis of unchanged files.
- **text-editor:** To generate precision patches.
- **shell:** To run tests (carbon dating/verification).

## The Archaeology Loop

### Phase 1: Surveying (The Mapper)
1. **Wake & Scan:** On startup or schedule, scan the file system.
2. **Delta Check:** Compare file modification times/hashes against the Memory Graph.
3. **Indexing:** For new/changed files, parse for symbols (classes, functions, imports).
4. **Graph Update:** Update `memory` with new nodes and relations (`Function A calls Function B`).
   - *Optimization:* Use `grep` to find usages of changed symbols to update incoming edges.

### Phase 2: Excavation (The Analyst)
1. **Hypothesis Generation:** Query the graph for anomalies:
   - *Dead Code:* Nodes with 0 incoming edges (excluding public API entry points).
   - *Complexity:* Files with high node density or large line counts.
   - *Deprecation:* Usage of known legacy libraries (e.g., `import requests` in an `aiohttp` project).
2. **Validation:**
   - Run `grep` to ensure no dynamic references exist.
   - (Optional) Run specific tests associated with the module.

### Phase 3: Curation (The Proposer)
1. **Create Exhibit:** Create a directory `workspace/dig_sites/site_[ID]_[Topic]/`.
2. **Draft Artifacts:**
   - `README.md`: "We found a likely dead function \`processOldData\` in \`legacy.py\`."
   - `evidence.txt`: Grep results showing zero usages.
   - `graph_view.json`: Subgraph of dependencies.
   - `refactor.patch`: A unified diff removing the function.
3. **Notification:** Append a headline to `workspace/daily_digest.md`.

### Phase 4: Preservation (The Learner)
1. **Human Feedback:**
   - If human applies patch: Mark node as `Excavated` in Memory.
   - If human deletes site without applying: Mark node as `Protected` (False Positive).
2. **Graph Pruning:** Remove `Excavated` nodes from the graph.

## Memory Schema
- **Entities:** `File`, `Class`, `Function`, `Variable`, `Module`.
- **Relations:** `imports`, `defines`, `calls`, `inherits_from`, `reads`.
- **Observations:** `last_scanned_hash`, `complexity_score`, `protection_status`.

## Safety & Recovery
- **Non-Destructive:** The agent only writes to its `workspace/dig_sites` folder. It never touches source code directly.
- **Staleness Check:** Patches include hash verification. If the source file changes between proposal and application, the patch fails safely.
- **Persisted State:** If the agent crashes, the Memory Graph allows it to resume scanning exactly where it left off.

## Example Workflow
1. Agent starts, scans a 10-year-old Python repo.
2. Identifies `utils.helpers.xml_parser` is never imported.
3. Creates `workspace/dig_sites/001_dead_xml_parser/`.
4. Human checks folder, sees `evidence.txt` confirms no usages.
5. Human runs `git apply workspace/dig_sites/001_dead_xml_parser/refactor.patch`.
6. Codebase is cleaner. Agent updates graph on next run.
