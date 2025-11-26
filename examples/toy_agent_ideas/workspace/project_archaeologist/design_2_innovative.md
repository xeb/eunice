# Design 2: The Graph Gardener (Innovative)

## Purpose
An autonomous background daemon that builds and maintains a live, semantic Knowledge Graph of the codebase. It doesn't just document; it "understands" the system, linking code entities to business concepts, commit history, and external library knowledge. It acts as an oracle for other agents.

## Loop Structure
**Trigger:** File system watcher (on change) or scheduled interval.
1. **Observe:** Detects changed files using `git status` or file hash comparison.
2. **Ingest:** Reads changed code and diffs.
3. **Reason:**
   - Identifies entities: Functions, Classes, API Endpoints, Database Tables.
   - Identifies relations: `calls`, `inherits_from`, `modifies_table`, `introduced_in_commit`.
4. **Update Graph:** Uses `memory` tools to update nodes and edges.
   - Example: `Function:ProcessPayment` -> `calls` -> `API:Stripe`.
5. **Enrich:** Periodically uses `web` to search for CVEs or documentation for imported libraries and attaches this metadata to the graph nodes.
6. **Prune:** Removes obsolete nodes (deleted code).

## Tool Usage
- **memory:** The core storage. Stores the graph of the codebase.
- **filesystem:** Reading source code.
- **web:** Looking up library documentation and security vulnerabilities.
- **grep:** Semantic search to find usage patterns across the codebase.

## Memory Architecture
- **Knowledge Graph (Native):** The agent's entire world model is stored in the MCP Memory server.
- **Ontology:**
  - **Nodes:** `File`, `Function`, `Class`, `Commit`, `Library`, `Concept`.
  - **Edges:** `contains`, `imports`, `calls`, `documents`, `fixes`.

## Failure Modes
- **Graph Bloat:** The graph becomes too large/noisy with irrelevant details (e.g., local variables).
- **Stale Links:** Code changes but the graph update fails, leading to hallucinated relationships.
- **Recovery:** A "Reindex" mode that wipes the memory graph and rebuilds from scratch.

## Human Touchpoints
- **Query:** Humans ask questions via a chat interface ("What functions call the UserDB?").
- **Correction:** Humans can manually annotate nodes in the graph (e.g., "This function is deprecated").
