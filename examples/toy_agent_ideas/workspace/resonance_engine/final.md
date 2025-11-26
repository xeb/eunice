# Agent: The Resonance Engine

## Abstract
**The Resonance Engine** is a background daemon that combats "Digital Hoarding" not by deleting files, but by transforming a static archive into a "Living Museum". It actively finds connections between your *current* focus and your *forgotten* past work, creating ephemeral "Exhibitions" (folders of symlinks) that surface relevant cold data.

## Core Mandates
1.  **Never Delete:** The agent never deletes source files. It only creates pointers (symlinks) and meta-documents.
2.  **Context is King:** Resurfacing is only valuable if relevant to *now*. Random resurfacing is a fallback, not the primary loop.
3.  **Privacy:** File contents are analyzed locally. Only extracted keywords are sent to Web Search for "Relevance Checking" (if enabled).

## Component Architecture

### 1. The Indexer (Filesystem + Grep)
*   **Role:** Crawl allowed directories during idle time.
*   **Action:** extract metadata (mtime, type) and "fingerprints" (TF-IDF keywords via `grep`).
*   **Output:** Updates the **Memory Graph**.

### 2. The Association Matrix (Memory)
*   **Graph Structure:**
    *   `Node:File` (path, hash)
    *   `Node:Concept` ("Machine Learning", "Tax Returns 2018")
    *   `Node:TimeCluster` (Events grouped by temporal proximity)
*   **Logic:**
    *   Link Files to Concepts.
    *   Link Files to TimeClusters (e.g., "All photos from May 2019 Trip").

### 3. The Curator (Web + Shell)
*   **Role:** Determine *what* to show today.
*   **Triggers:**
    *   **Contextual:** Watch `~/.recent_files`. If user opens "project_alpha_v2.txt", find "project_alpha_v1_notes.txt" (from 3 years ago).
    *   **Temporal:** "On this day 5 years ago..."
    *   **External:** Fetch "Trending Tech Topics". If "Rust" is trending, find local "Rust" PDFs.
*   **Action:**
    *   Create `~/Desktop/Resonance/Exhibit_[Topic]`.
    *   Symlink the identified files.
    *   Write `curator_note.md`: "You are working on [X]. These files from 2019 seem related because they mention [Y] and [Z]."

## The Execution Loop
```bash
while true; do
  # 1. Update Context
  CURRENT_FOCUS=$(tail -n 1 ~/.bash_history | extract_keywords)

  # 2. Query Memory for Resonance
  # Find cold files (>1y old) related to CURRENT_FOCUS
  RESONANT_FILES=$(memory_query "match (f:File)-[:HAS_TOPIC]->(t:Topic) where t.name = '$CURRENT_FOCUS' and f.mtime < '2024-01-01' return f")

  # 3. Publish Exhibition
  if [ ! -z "$RESONANT_FILES" ]; then
     EXHIBIT_DIR="~/Desktop/Resonance/Related_to_$CURRENT_FOCUS"
     mkdir -p "$EXHIBIT_DIR"
     for file in $RESONANT_FILES; do
       ln -s "$file" "$EXHIBIT_DIR/"
     done
     notify-send "The Resonance Engine" "Found 3 forgotten files related to your current work."
  fi

  sleep 3600 # Check hourly
done
```

## Failure Modes & Recovery
*   **Concept Drift:** User is no longer interested in "React.js" but has thousands of files on it.
    *   *Mitigation:* User can "Dismiss" an exhibition. The agent adds a `[:IGNORED_TOPIC]` edge to the graph to suppress future resurfacing.
*   **Broken Links:** User moves source files.
    *   *Mitigation:* The Indexer re-hashes files. If a path changes but hash matches, it updates the graph node instead of creating a new one (Self-Healing).

## Future Expansion
*   **Visual Similarity:** Use `filesystem_read_media_file` + local embedding model to link images by content, not just date.
*   **Web Archiving:** If a local bookmark file is found, check if the URL is dead. If so, fetch the Wayback Machine version and save it locally.

