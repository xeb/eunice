# Final Design: The Contextual Cartographer (Project Archaeologist)

## Synthesis
After reviewing the Conservative (Librarian), Innovative (Graph Gardener), and Hybrid (Active Surveyor) approaches, the best immediate architecture for an MCP-based agent is a **refined version of the Graph Gardener**.

The "Librarian" is too passive and doesn't leverage the full power of agentic reasoning. The "Active Surveyor" is too risky for a general-purpose tool (executing code is dangerous).

The **Contextual Cartographer** focuses on building a queryable, persistent mental model of the codebase that evolves. It uses the file system as the "Truth" and the Memory Graph as the "Index/Understanding".

## Core Loop
1. **Discovery (FileSystem + Grep):**
   - The agent recursively maps the directory structure.
   - It identifies "Landmarks": Config files, READMEs, Entry points (main.py, index.js).
   - It uses `grep` to find cross-references (imports, API calls).

2. **Cartography (Memory):**
   - Instead of just storing "File A imports File B", it creates higher-level entities.
   - **Concept Nodes:** "Authentication", "Payment Processing", "Legacy Data Sync".
   - **Entity Nodes:** Specific files or classes.
   - **Relation:** Links Concepts to Entities.
   - *Key Innovation:* The agent searches for "Implicit Links" (e.g., string references to database tables, API routes defined in one file and called in another) which static analysis often misses.

3. **Enrichment (Web):**
   - For every 3rd party library found, it fetches the summary, license, and primary purpose from the web.
   - This provides context: "This obscure import is actually a PDF generation library."

4. **Interface (Chat/Files):**
   - Users can query: "Where is the PDF logic?" -> Agent looks up "PDF" concept in Memory -> Returns list of files and verified libraries.
   - Agent maintains a `CODEX.md` in the root: A human-readable summary of its graph.

## Architecture Details
- **Persistence:**
  - **Primary:** MCP Memory Graph (Graph database).
  - **Backup:** `workspace/knowledge_dump.json` (for portability).
- **Tools:**
  - `filesystem`: Read-only access to source.
  - `memory`: Read/Write access to the graph.
  - `web_brave_search`: For library/API context.
  - `grep`: For deep pattern matching.

## Safety & Failure Handling
- **Hallucination Check:** Before answering "Function X does Y", the agent must re-read the file to confirm it still exists (Lazy Verification).
- **Graph Drift:** A "Rescan" command runs weekly to validate all edges in the graph against the file system.

## Why This Wins
It balances **safety** (read-only on code) with **utility** (deep, semantic understanding). It solves the "Context Rot" problem by decoupling the *understanding* from the *code* but keeping them linked via the graph.
