# Design 2: The RPG Overlay (Experimental)

## Purpose
To transform the software development lifecycle (SDLC) into a persistent Role-Playing Game, where the codebase is the "Dungeon" and technical debt manifests as "Monsters" with hit points.

## Loop Structure
1.  **World Generation:**
    *   Maps the directory structure to "Zones" (e.g., `src/auth` = "The Gate of Authentication").
    *   files > 500 lines are "Elites".
    *   files > 1000 lines are "Bosses".
2.  **Combat Loop (The "Boss Fight"):**
    *   **Boss:** `LegacyController.ts` (2000 lines, High Cyclomatic Complexity).
    *   **HP:** 2000 (1 HP per line).
    *   **Action:** A developer submits a PR removing 50 lines.
    *   **Reaction:** Agent calculates damage (`2000 - 50 = 1950 HP`).
    *   **Event:** "You dealt 50 damage to the LegacyController! It flinches."
3.  **Character Progression:**
    *   Devs have "Classes" based on their commit history (detected via `git log`).
    *   **Paladin:** Fixes security bugs.
    *   **Rogue:** Deletes dead code.
    *   **Bard:** Writes documentation.
    *   **Cleric:** Increases test coverage.
4.  **Loot Drop:**
    *   Upon "killing" a Boss (refactoring file < 500 lines), the agent generates a unique ASCII Badge or "Item" (e.g., "The Sword of Clean Code") and adds it to the user's profile in `PLAYERS.md`.

## Tool Usage
*   `memory`: Stores the "World State" (Current HP of all files, Player Stats, Active Raids).
*   `shell`: Runs complexity analysis tools (e.g., `complexity-report` or simple line counts).
*   `filesystem`: Updates the "Game Log" and player profiles.

## Memory Architecture
*   **Entity Graph:**
    *   `Node: File` (Properties: HP, Level, Type)
    *   `Node: Player` (Properties: XP, Class, Inventory)
    *   `Relation: DAMAGED_BY` (Player -> File)

## Failure Modes
*   **Grinding:** Users making trivial commits to farm XP.
    *   *Fix:* Diminishing returns on small commits.
*   **Boss Regeneration:** Someone adds code to a refactored file.
    *   *Event:* "The LegacyController RESURRECTS with 500 HP!"

## Human Touchpoints
*   **Opt-in:** Users register via a config file.
*   **Narrative:** The agent posts "Quest Updates" as PR comments.
