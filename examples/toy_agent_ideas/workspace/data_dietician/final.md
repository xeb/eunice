# Agent: The Data Dietician

## System Role
The Data Dietician is an autonomous "Digital Metabolism" agent. It operates on the principle that **storage is transient, but knowledge is permanent**. It proactively manages filesystem bloat not just by deleting files, but by "digesting" them—transforming unused, heavy documents into lightweight, structured knowledge graph nodes and summary artifacts.

## Problem Domain
**Digital Hoarding & Storage Optimization**: Users accumulate terabytes of "read-once" data (PDFs, logs, reference images, old projects) that they are afraid to delete "just in case." This leads to cognitive overload, search friction, and wasted resources.

## Key Insight
**Semantic Compression Lifecycle**: Data should not be binary (Exists vs. Deleted). It should exist on a gradient of fidelity:
1.  **High Fidelity:** The original raw file (Active).
2.  **Medium Fidelity:** A lossy compressed version or a link to a public source (Dormant).
3.  **Low Fidelity:** A structured Knowledge Graph node + Text Summary (Archived).
4.  **Zero Fidelity:** Deleted (Forgotten).

The agent moves files down this gradient based on "Metabolic Rates" (access frequency).

## Core Loop
1.  **Auditing Phase (The Senses):**
    *   Scans the filesystem using `shell` tools (`find`, `du`) to identify "Atrophied" files (large size, last accessed > ).
    *   Checks `memory` to see if this file has already been indexed.

2.  **Digestion Phase (The Stomach):**
    *   For each atrophied file:
        *   **Check Public Availability:** Uses `web` search to see if the file exists on a stable public URL (e.g., arXiv, GitHub). If so, immediate replacement with a link.
        *   **Extract Knowledge:** Reads the file content. Extracts:
            *   *Summary*: A 1-paragraph gist.
            *   *Entities*: Key people, technologies, or concepts.
            *   *Intent*: Why was this saved?
        *   **Store in Memory:** Creates/Updates `memory` entities linked to the original filename.

3.  **Metabolism Phase (The Muscle):**
    *   **Destructive Action:** Deletes the original heavy file.
    *   **Create Tombstone:** Writes a small Markdown file (`[OriginalName].diet.md`) in its place.
    *   *Tombstone Content:*
        *   "This file was digested on [Date]."
        *   Summary of content.
        *   Public Source Link (if found).
        *   Extracted Entities.
        *   "To restore, download from [Link] or check backup."

4.  **Recall Phase (The Brain):**
    *   When the user searches `memory` or `grep`s the Tombstone, they find the *knowledge* without needing the *bytes*.

## Tool Usage
*   **Memory Server:**
    *   `memory_create_entities`: To store the "Soul" of the file (summary, entities).
    *   `memory_search_nodes`: To check if a file is already known.
*   **Filesystem Server:**
    *   `filesystem_read_file`: To ingest content for digestion.
    *   `filesystem_write_file`: To create Tombstones.
    *   `filesystem_move_file`: To move files to a `.trash` buffer before final deletion.
*   **Web Server:**
    *   `web_brave_web_search`: To find public mirrors of local files.
*   **Shell Server:**
    *   `find`, `du`, `gzip`: For efficient filesystem traversal and compression.

## Autonomy Level
**High Autonomy (Daemon)** with **Safety Buffers**.
The agent runs in the background. It moves files to a "Digestive Tract" (Trash folder) for 30 days before final deletion, giving the user a window to undo.

## Persistence Strategy
*   **Filesystem:** Holds the "Current State" (hot files) and "Tombstones" (pointers).
*   **Memory Graph:** Holds the "Long-term Knowledge" extracted from cold files. Even if the file is gone, the *fact* that it existed and *what it said* is preserved.

## Failure Modes & Recovery
*   **False Positive Digestion:** The agent digests a file the user needed (e.g., a tax document).
    *   *Recovery:* The "Tombstone" contains enough metadata to potentially find it again, and the `.trash` buffer allows undoing for 30 days.
*   **Extraction Failure:** The agent summarizes a spreadsheet but misses a critical row.
    *   *Recovery:* User defines "Protected" directories (e.g., `/Documents/Financial`) that are never digested.
