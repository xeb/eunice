# Agent: The World-Smith

## Abstract
The World-Smith is an autonomous background daemon for creative writers that maintains the "Story Bible" of a fictional universe. It observes the author's manuscript changes in real-time, extracting entities (characters, locations, lore) into a persistent Knowledge Graph, and auto-generating a human-readable Wiki. It simultaneously acts as a continuity linter, flagging contradictions (e.g., eye color changes, dead characters speaking) and timeline errors.

## Core Toolset
- **memory**: Stores the authoritative "World Graph" (Relations, Timelines, Hidden State).
- **filesystem**: 
    -   Reads: Manuscript files (Markdown/Scrivener exports).
    -   Writes: `wiki/` (Character sheets, Location details) and `reports/` (Consistency checks).
- **grep**: Performs high-speed context searches across the entire project history.
- **web**: Fetches real-world reference data (etymology, geography, physics) to ground the fiction.

## Architecture

### 1. The Observation Loop (Daemon)
The agent runs a file watcher on the manuscript directory.
-   **Trigger**: File save detected in `Drafts/Chapter_*.md`.
-   **Action**: 
    1.  Read the modified text.
    2.  **Entity Extraction**: Identify proper nouns, physical descriptions, and assertions.
    3.  **Graph Update**: Update the internal memory graph with new observations (tagged by Chapter/Scene).

### 2. The Wiki Generator (Archivist)
Unlike opaque database tools, The World-Smith maintains a transparent, flat-file Markdown wiki.
-   **Sync**: Periodically (or on demand), the agent serializes the Memory Graph into the `wiki/` folder.
    -   `wiki/People/Alice.md`
    -   `wiki/Places/Zarth.md`
-   **Enrichment**: If a wiki entry is sparse, the agent uses **Web Search** to suggest details (e.g., "Name origin of Zarth," "Climate of mountainous regions") and appends them as "Suggestions".

### 3. The Continuity Linter (Auditor)
Before a "Release" or on command, the agent runs a full consistency pass.
-   **Logic**:
    -   *Trait Stability*: "Value X for Entity Y changed from A to B without explanation."
    -   *Temporal Logic*: "Event B (Chapter 5) references Event A (Chapter 2), but Event A is dated later."
    -   *State Tracking*: "Character Z is marked 'Dead' in Chapter 3 but speaks in Chapter 7."
-   **Output**: `reports/Lint_2025-11-25.md`

## Persistence Strategy: The "Twin-State" Model
1.  **Memory Graph (The Database)**: Optimized for query speed and logic. Handles the "relations" and "hidden state."
2.  **Filesystem Wiki (The Interface)**: Optimized for human readability and editing. 
    -   *Bidirectional Sync*: If the user manually edits `wiki/People/Alice.md` to change her eyes to Green, the Agent reads this change and updates the Memory Graph, effectively "patching" the world truth.

## Autonomy Level
**Semi-Autonomous / Background Daemon**
-   **Read/Analyze**: Fully Autonomous.
-   **Wiki Generation**: Fully Autonomous (Additions/Suggestions).
-   **Manuscript Editing**: **Never**. The agent never touches the draft. It only comments or reports.

## Failure Modes & Recovery
1.  **Entity Resolution Errors**: Confusing "The Baker" (Job) with "Baker" (Surname).
    -   *Fix*: User adds an alias in the Wiki file metadata: `aliases: ["Mr. Baker", "The Bread Maker"]`.
2.  **Hallucinated Lore**: Agent infers a character is angry when they are just tired.
    -   *Fix*: User deletes the incorrect line in the Wiki; Agent learns to trust the Wiki over its inference.
3.  **Spoiler Leaks**: Agent reveals a secret in the Wiki that shouldn't be known yet.
    -   *Fix*: Wiki files support `secrets: [ ... ]` blocks that are hidden from the general view or marked as "DM Only".

## Example Workflow
1.  Author writes: *"Alice drew her obsidian dagger."*
2.  Agent scans, sees `Alice` owns `obsidian dagger`.
3.  Agent checks Graph: "Alice lost her dagger in Chapter 2."
4.  Agent alerts: "Continuity Error: Alice lost this item in Ch2."
5.  Author corrects text OR Author updates Wiki: *"Alice bought a replacement dagger off-screen."*
