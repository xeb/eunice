# Design 2: The Biographer (Experimental)

## Purpose
An autonomous "digital second brain" that organizes files by *meaning* and *context*, constructing a rich narrative of the user's digital life. It treats the filesystem as a fluid, semantic web.

## Loop Structure
1. **Discovery:** Recursively crawls the entire home directory (excluding system folders).
2. **Deep Analysis:**
   - Reads text content of documents.
   - Summarizes content using LLM.
   - Extracts entities (People, Places, Events, Topics).
3. **Graph Enrichment:**
   - Stores entities in the **Memory MCP** graph.
   - Links files to entities (e.g., `contract.pdf` -> `Relation:MENTIONS` -> `Company X`).
   - Uses **Web Search** to find context for entities (e.g., "Who is Company X?" -> "Tech startup in SF").
4. **Dynamic Reorganization:**
   - Uses `filesystem_move_file` to cluster files into semantic folders like `Projects/Project Alpha/Financials` even if the file was originally named `scan001.pdf`.
   - Renames files based on content summary (e.g., `Invoice_CompanyX_Nov2024.pdf`).

## Tool Usage
- **Memory MCP:** 
  - `memory_create_entities` to store People, Companies, Projects.
  - `memory_create_relations` to link Files to these concepts.
  - `memory_search_nodes` to find related files for clustering.
- **Filesystem:** Full read/write access to reorganize the user's life.
- **Web Search:** To resolve ambiguities (e.g., is "Python" the snake or the language? Search helps clarify context).

## Memory Architecture
- **Graph-First:** The "truth" of the file system lives in the Memory graph. The physical file system is just a projection of this graph.
- **Embeddings:** (Implicit) Uses the LLM's understanding to group semantically similar items.

## Failure Modes
- **Hallucination:** Might misclassify a receipt as a love letter if keywords overlap.
- **Over-shuffling:** Moving files too frequently as context changes, confusing the user.
- **Privacy:** Sending private file content to external model APIs (mitigated by local LLM if available, but risk exists).

## Human Touchpoints
- **Query Interface:** User asks "Where are my tax documents?" and agent queries the graph to find them, regardless of folder location.
- **Feedback:** User can "lock" folders to prevent the agent from touching them.
