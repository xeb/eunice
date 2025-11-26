# Design 3: The Curator (Hybrid)

## Purpose
A "Shadow File System" approach. The agent builds a rich metadata layer (Knowledge Graph) without disturbing the physical file location unless explicitly asked. It creates "Virtual Views" (symlink trees or HTML dashboards) to offer organized perspectives.

## Loop Structure
1. **Indexing:** Monitors file system events (or periodic scan).
2. **Metadata Extraction:** 
   - Extracts technical metadata (size, date) locally.
   - Extracts semantic metadata (keywords, summary) via LLM for text-heavy files.
3. **Graph Building:**
   - Updates **Memory MCP** with file nodes and extracted metadata properties.
   - Identifies "Clusters" (e.g., "Trip to Japan") based on time and content proximity.
4. **Virtual Organization:**
   - Creates a `Virtual_Views/` directory.
   - Inside, creates folder structures like `By_Topic/Finance`, `By_Person/Alice` using **symlinks** pointing to the real files.
   - This allows multiple distinct organizational structures to coexist without duplicating data.
5. **Suggestion:** periodically suggests "Cleanup Actions" (e.g., "You have 5 duplicates of `resume.pdf`, delete 4?").

## Tool Usage
- **Filesystem:** 
  - Read access for indexing.
  - Write access *only* for the `Virtual_Views/` directory and log files.
- **Memory MCP:** Stores the "Shadow Filesystem" state (paths, hashes, tags, relationships).
- **Web Search:** Used sparingly to auto-tag public documents (e.g., research papers).

## Memory Architecture
- **Dual-State:**
  - **Physical State:** Recorded path on disk.
  - **Semantic State:** Nodes in the Knowledge Graph.
- The agent constantly reconciles the two. If a file is moved by the user, the agent updates the graph.

## Failure Modes
- **Stale Links:** If the user deletes a file, symlinks break. Agent must run "Dead Link Cleanup".
- **Graph Drift:** If the agent isn't running, it misses changes. Needs a "Resync" mode on startup.

## Human Touchpoints
- **Dashboard:** A simple generated HTML page or Markdown file `My_Archive.md` that lists "Recent Topics" and links to files.
- **Approval:** Cleanup actions (deletions, huge moves) require user confirmation via a CLI prompt or config flag.
