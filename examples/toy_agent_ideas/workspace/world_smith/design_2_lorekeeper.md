# Design 2: The Lore Keeper

## Purpose
A proactive "World Librarian" that autonomously maintains a "Story Bible" wiki based on the manuscript. As the author writes, the agent updates character sheets, location descriptions, and timelines in a separate `wiki/` folder.

## Core Toolset
- **filesystem**: Watch manuscript folder, read/write Wiki files.
- **memory**: Intermediate graph to store entity states before serializing to Markdown.
- **web**: Fetch etymology, historical references, or scientific facts to flesh out lore entries.

## Loop Structure
1.  **Watch**: Monitor the manuscript folder for changes.
2.  **Ingest**: When a chapter is saved, parse it for new information.
3.  **Update**:
    -   If a new character appears, create `wiki/characters/Name.md`.
    -   If an existing character reveals new backstory, append to their file.
4.  **Enrich**: (Optional) If the user writes "They traveled to the Obsidian Peaks," the agent can draft a lore entry for "Obsidian Peaks" suggesting geology/climate based on similar real-world locations found via Web search.

## Memory Architecture
-   **Hybrid**:
    -   **Graph**: Short-term working memory for conflict resolution.
    -   **Filesystem**: Long-term storage in human-readable Markdown (The Wiki).
    -   *Why?* Allows the author to edit the memory directly (by editing the wiki).

## Failure Modes
-   **Hallucination**: Inventing lore that the author didn't intend (e.g., assuming a character is a villain because they frowned).
-   **Spoiler Contamination**: Revealing a twist too early in a character's public bio.
-   **Recovery**: Git version control on the `wiki/` folder allows rolling back bad agent updates.

## Human Touchpoints
-   **Co-Creation**: The author writes the story, the agent writes the wiki. The author can edit the wiki to "correct" the agent's understanding.
-   **Inquiries**: The author can leave comments in the manuscript like `<!-- @LoreKeeper: What is the capital of Zarth? -->` and the agent replaces it with the answer.
