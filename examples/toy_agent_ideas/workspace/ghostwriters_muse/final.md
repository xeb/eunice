# Agent: The Contextual Muse

## Purpose
To surround the creative writer with a "halo" of relevant facts, sensory inspiration, and continuity checks, turning the writing environment into an active partner rather than a passive canvas. It solves "Writer's Block" not by writing *for* you, but by feeding your imagination and memory.

## Core Loop
1.  **Monitor:** The agent watches the project directory for file modifications (`.md` files).
2.  **Ingest & Graph:**
    *   Reads the modified text.
    *   Extracts **Entities** (Characters, Places) and updates the **Memory Graph**.
    *   **Consistency Check:** If an entity's new description contradicts the graph (e.g., "Alice's blue eyes" vs "Alice's brown eyes"), it flags an alert.
3.  **Contextual Research:**
    *   Identifies **Key Terms** (historical events, technical jargon) and **Settings** (locations, moods).
    *   **Web Search:** Fetches definitions, dates, or mechanical details using `web_brave_web_search`.
    *   **Visual Search:** Fetches mood/reference images using `web_brave_image_search`.
4.  **Update Sidecar:**
    *   Instead of modifying the draft, it updates a `_muse.md` file side-by-side with the draft.
    *   The sidecar contains:
        *   **⚠️ Consistency Alerts**
        *   **📚 Research Notes** (Summarized facts)
        *   **🎨 Mood Board** (Thumbnails/Links to images)

## Tool Usage
*   **filesystem:** `filesystem_read_text_file` (Watch), `filesystem_write_file` (Update Sidecar).
*   **memory:** `memory_create_entities`, `memory_search_nodes` (Consistency/World Bible).
*   **web:** `web_brave_web_search` (Facts), `web_brave_image_search` (Visuals).
*   **fetch:** `fetch_fetch` (Download assets for offline viewing).

## Persistence Strategy
*   **Hybrid:**
    *   **Memory Graph:** Holds the "Truth" of the story (facts about the world).
    *   **Filesystem:** Holds the "Interface" (Sidecar files) and "Assets" (Images).

## Autonomy Level
*   **Background Daemon:** Runs continuously in the background.
*   **Zero-Click Interface:** User never commands the agent; the agent *reacts* to the user's writing.

## Failure Modes & Recovery
*   **Graph Poisoning:** Agent misinterprets a dream sequence as fact.
    *   *Recovery:* User can edit the `_world_bible.md` (an export of the graph) to correct facts.
*   **Search Noise:** Irrelevant images.
    *   *Recovery:* User ignores the sidecar; agent clears/refreshes it on the next scene change.

## Key Insight
Separating the **"Editor"** (Consistency) and **"Researcher"** (Inspiration) from the **"Writer"** (Drafting) allows the agent to be helpful without being intrusive. Using a **Memory Graph** for fiction continuity is a novel application of knowledge graph technology usually reserved for enterprise data.
