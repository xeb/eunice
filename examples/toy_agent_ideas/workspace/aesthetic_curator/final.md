# Final Design: The Aesthetic Curator

## Purpose
A "Visual Subconscious" for creative professionals. This background agent monitors active creative projects (writing, game design, code) and proactively builds a local, navigable "Museum of Inspiration" that evolves with the work. It bridges the gap between *Semantic Intent* (words in a draft) and *Visual Reference* (images/videos), using a persistent Graph to map the two.

## Core Loop: The "Semantics-to-Esthetics" Cycle

1.  **Observation (Filesystem + Grep)**
    *   The agent watches a designated `projects/` directory.
    *   It extracts **Entities** (Places, Characters) and **Descriptors** (Adjectives, Emotions) from changed files (e.g., Markdown, Fountain scripts, Design Docs).
    *   *Example*: Finds "The neon rain slicked the cybernetic arm" in `chapter3.md`.

2.  **Expansion (Memory + Web)**
    *   It queries the **Memory Graph**: "Do we have visuals for 'Neon Rain' or 'Cybernetics'?"
    *   If nodes are weak or missing, it performs **Semantic Expansion**:
        *   Search Web: "Visual style of neon rain", "Cyberpunk aesthetic tropes".
        *   Result: Adds related concepts "Blade Runner", "Synthwave", "Reflections" to the graph.

3.  **Acquisition (Web + Fetch)**
    *   It performs targeted `web_brave_image_search` for the new concepts.
    *   It downloads assets to a hidden `.cache/aesthetic_curator/` folder.

4.  **Curation (Shell + Filesystem)**
    *   **Palette Check**: Uses `shell` (ImageMagick) to analyze the image's color palette. Does it match the project's defined "Mood"? (e.g., Dark/Cyan/Magenta).
    *   **Deduplication**: Hashes images to prevent duplicates.
    *   **Promotion**: Moves passing images to `projects/<name>/moodboard/assets/`.

5.  **Presentation (Filesystem)**
    *   Generates a dynamic `moodboard.html` (or `gallery.md`) in the project folder.
    *   **Key Feature**: "Context Links". The HTML shows the image *next to the text snippet* that spawned it.
    *   *User Interaction*: User can "Star" (reinforce link strength) or "Trash" (sever link) images via the HTML interface (which writes to a local log file the agent reads).

## Architecture Details

### The Aesthetic Knowledge Graph
Stored in `memory`, the graph structure is:
*   **Nodes**:
    *   `Concept` ("Cyberpunk", "Victorian")
    *   `VisualElement` ("Fog", "Gear", "Neon")
    *   `Project` ("MyNovel")
    *   `Asset` (File path to image)
*   **Edges**:
    *   `Project --[HAS_MOOD]--> Concept`
    *   `Concept --[VISUALLY_RELATED_TO]--> VisualElement`
    *   `Asset --[EXEMPLIFIES]--> VisualElement`

### Tool Chain
*   **Web**: `web_brave_image_search` (finding raw pixels), `web_brave_web_search` (finding aesthetic vocabulary).
*   **Memory**: Tracking the "Visual Language" and user preferences (what they trash vs keep).
*   **Shell**: `magick` for color extraction/thumbnailing; `grep` for scanning large text corpuses.
*   **Filesystem**: storage of heavy assets and generation of the UI (HTML/MD).

## Failure Modes & Recovery
1.  **"Visual Noise" (Bad Search Results)**
    *   *Problem*: "Apple" search returns fruit instead of tech.
    *   *Recovery*: Agent analyzes the *other* words in the user's text ("circuit", "screen") to refine the search query ("Apple computer vintage").
2.  **Disk Bloat**
    *   *Problem*: 10GB of images.
    *   *Recovery*: The agent implements a "Garbage Collection" policy—assets not "Starred" or "Viewed" in 30 days are deleted from the cache.
3.  **Copyright/License**
    *   *Problem*: Using copyrighted images.
    *   *Recovery*: Agent defaults to searching Creative Commons/Public Domain, or tags images as "Reference Only - Do Not Publish" in metadata.

## Novelty & Insight
Most moodboard tools are **Top-Down** (User searches -> Image). The Aesthetic Curator is **Bottom-Up** (Text/Code -> Concept -> Image). It treats *textual descriptions* as implicit queries for a visual search engine, creating a feedback loop where your writing generates its own art direction automatically.
