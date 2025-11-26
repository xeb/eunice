# Design 3: The Semantic Metabolist (Experimental/Aggressive)

## Purpose
To treat data storage as a biological system with a metabolism. Information that is not used is not just deleted, but "digested"—transformed from high-fidelity (heavy) formats into low-fidelity (light) knowledge graph nodes. The file disappears, but the *information* remains.

## Core Loop
1. **Triage:** Identifies "Atrophied" data (files untouched for > UserDefinedThreshold).
2. **Digest:** 
   - Reads the file contents.
   - Extracts entities, summaries, and key facts.
   - Stores these structured facts in the `memory` graph.
3. **Metabolize:**
   - **Deletes** the original heavy file.
   - **Excretes** a "Tombstone" file: `[Filename].metabolized.md`.
   - The Tombstone contains: The summary, the extracted entities, the creation/access dates, and a query to the Memory Graph to retrieve related context.
4. **Recall:** If the user opens the Tombstone, they get the gist. If they need the original, they must request a "Restoration" (which might involve searching online or checking backups, as the local copy is gone).

## Tool Usage
- `text-editor`: To read chunks of large files.
- `memory`: The primary storage. The "File" becomes a "Memory Node".
- `filesystem`: Deletion and creation of Markdown summaries.
- `grep`: To search for keywords inside files before digestion to ensure nothing critical is lost.

## Memory Architecture
- **Concept:** The File System *is* the temporary cache. The Memory Graph *is* the permanent store.
- **Nodes:** `Concept`, `Project`, `Event` (extracted from files).
- **Edges:** `MENTIONED_IN`, `RELATED_TO`.

## Failure Modes
- **Data Loss:** The summary misses a crucial detail (e.g., a specific number in a spreadsheet).
- **Irreversibility:** Once metabolized, the original byte-perfect file is gone.
- **Mitigation:** A "Trash Can" holding period where metabolized files sit in a `.trash` folder for 30 days before true deletion.

## Human Touchpoints
- **Configuration:** Setting the "Metabolism Rate" (e.g., "Digestion starts after 6 months of inactivity").
- **Tombstone Interaction:** Reading the summary files.
