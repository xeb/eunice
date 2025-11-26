# Design 1: The Librarian (Conservative)

## Purpose
A passive, privacy-first agent that indexes local files and builds a rich semantic graph to answer "What do I have on X?" queries with high precision, including forgotten "cold" data. It focuses on **Digital Preservation** rather than interference.

## Core Toolset
*   **filesystem:** Read-only access to user documents (excluding system dirs).
*   **memory:** Stores the 'Catalog' (File paths -> Keywords -> Concepts).
*   **grep:** For precise content searching without sending data to external APIs.

## Loop Structure
1.  **Indexing (Nightly):**
    *   Walk directory tree.
    *   For each file, check mtime and hash.
    *   If new/changed: Run extraction logic (grep for frequent terms).
    *   Update Memory Graph: `File(name) -> contains -> Term(keyword)`.
2.  **Query (On Demand):**
    *   User runs: `agent query "Machine Learning"`.
    *   Agent searches Memory Graph for "Machine Learning" node.
    *   Agent traverses edges to find connected Files.
    *   Agent returns list of files with context (e.g., "Modified 2018, contains 50 mentions").

## Memory Architecture
*   **Entities:** `File`, `Directory`, `Topic`, `Date`.
*   **Relations:**
    *   `File -> located_in -> Directory`
    *   `File -> created_on -> Date`
    *   `File -> mentions -> Topic`
    *   `Topic -> related_to -> Topic` (via co-occurrence in files)

## Failure Modes
*   **Permission Denied:** Skips file, logs error.
*   **Large Files:** Skips files > 10MB to prevent hanging.
*   **Stale Graph:** If files are moved by user, graph breaks. (Fix: Full rescan weekly).

## Human Touchpoints
*   **Setup:** User defines allowed directories.
*   **Interaction:** strictly Request-Response. No notifications.

## Pros/Cons
*   **Pros:** Safe, low resource usage, no "annoying" interruptions.
*   **Cons:** Doesn't solve "out of sight, out of mind". User must remember to ask.
