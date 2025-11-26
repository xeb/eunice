# Design 3: The Sensory Expander (Hybrid)

## Purpose
To transform abstract or "thin" writing into rich, immersive descriptions by providing multi-modal inspiration (images, sounds, sensory vocabulary). It combats "White Room Syndrome."

## Loop Structure
1. **Scene Detection:** The agent identifies when a new scene or setting is introduced (e.g., "They walked into the neon-lit bar").
2. **Expansion Query:** It generates search queries for:
    *   **Visuals:** `web_brave_image_search` ("cyberpunk bar interior neon").
    *   **Vocabulary:** `web_brave_web_search` ("words to describe the smell of rain," "sounds in a crowded market").
3. **Asset Gathering:** It downloads low-res reference images and lists of evocative adjectives/verbs.
4. **Mood Board Gen:** It creates a local `assets/` folder and generates a `mood_board.md` that displays these images and words alongside the draft.

## Tool Usage
*   **web:** `web_brave_image_search` for visuals, `web_brave_web_search` for thesaurus/sensory details.
*   **filesystem:** `filesystem_create_directory` for assets, `filesystem_write_file` for the mood board.
*   **fetch:** `fetch_fetch` to download images (if needed, or just link them).

## Memory Architecture
*   **Associative:** Uses the Memory Graph to link specific *files* or *chapters* to specific *moods* or *themes*. 
*   **Example:** `Chapter 5` --(has_mood)--> `Melancholy` --(associated_image)--> `rainy_window.jpg`.

## Failure Modes
*   **Distraction:** Images might derail the writer's imagination rather than fuel it. *Recovery:* User can toggle the "Visual Mode" off.
*   **Copyright:** Downloading random images. *Mitigation:* Use generic/stock search or only keep local references for private use.

## Human Touchpoints
*   **Inspiration:** The user glances at the mood board when stuck.
*   **Direction:** User can type `[Mood: Dark, Industrial]` to guide the image search.
