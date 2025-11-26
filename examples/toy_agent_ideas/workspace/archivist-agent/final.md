# Final Design: The Curator (Hybrid Personal Archivist)

## Executive Summary
The "Curator" is the optimal balance between safety and intelligence. It avoids the data loss risks of the "Biographer" (Experimental) while exceeding the limited utility of the "Librarian" (Conservative). By decoupling physical storage from logical organization via a "Shadow Knowledge Graph" and "Virtual Views" (symlinks), it provides the user with a powerful semantic file system without disrupting their existing workflow.

## Core Toolset
1. **Filesystem:** For reading content and managing `Virtual_Views/`.
2. **Memory:** For storing the Knowledge Graph (Entities: File, Topic, Person, Date; Relations: MENTIONS, AUTHORED_BY, CREATED_ON).
3. **Web Search:** For enriching entity metadata (e.g., identifying companies in invoices).

## Architecture

### 1. The Indexing Loop (Background Daemon)
- **Trigger:** Watchdog event or hourly cron.
- **Action:** 
  - Hash files to detect duplicates.
  - Check if file hash exists in Memory Graph.
  - If new:
    - Extract Text -> Summarize -> Extract Entities.
    - Store Node: `File(name, path, hash, summary)`.
    - Store Relations: `File -> MENTIONS -> Entity`.

### 2. The Semantic Layer (Memory MCP)
- Acts as the "Brain".
- Connects disparate files:
  - `invoice_2024.pdf` and `email_draft.txt` are linked because they both mention "Project Alpha".
- **Persistence:** The graph is saved to disk via the Memory MCP's native storage.

### 3. The Presentation Layer (Virtual Views)
- The agent maintains a folder `~/Virtual_Archive/`.
- **Dynamic Folders:**
  - `~/Virtual_Archive/By_Topic/Finances/` -> contains symlinks to tax PDFs, excel sheets, and receipts located anywhere on the drive.
  - `~/Virtual_Archive/By_Date/2024/Nov/` -> Chronological view.
- **Dashboard:** A `README.md` in the root of `Virtual_Archive` that dynamically updates with "Insights" (e.g., "You have 14 unread papers about AI Agents").

## Safety & Recovery
- **Read-Only on Source:** The agent never moves or deletes original files without explicit confirmation.
- **Symlink Safety:** If symlinks are deleted, data is safe. If data is moved, the agent heals the symlink on next scan.
- **Drift Detection:** On startup, the agent verifies all graph paths still exist on disk.

## Implementation Roadmap
1. **Phase 1:** Build the scanner and basic Memory Graph population.
2. **Phase 2:** Implement the Symlink generator for simple dates/types.
3. **Phase 3:** Integrate Web Search for entity enrichment (Company logos, descriptions).
4. **Phase 4:** Build the "Cleanup Suggestions" CLI.

## Why This Wins
- **Low Risk:** User data is never imperiled.
- **High Reward:** Immediate semantic search and organization.
- **Extensible:** The Knowledge Graph can later power a Chatbot ("Chat with my Archive").
