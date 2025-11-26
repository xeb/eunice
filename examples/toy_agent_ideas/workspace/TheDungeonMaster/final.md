# Agent Design: The Dungeon Master

## Executive Summary
**The Dungeon Master** is a "Gamification Engine" that runs as a background agent in a software repository. It transforms the often-tedious work of maintenance, refactoring, and documentation into a persistent Role-Playing Game (RPG). By reifying technical debt as "Monsters" with Hit Points and refactoring efforts as "Combat," it aligns developer incentives with long-term codebase health.

## Core Philosophy
"Codebases are not built; they are *explored* and *conquered*. Technical debt is not a chore; it is a *Dragon* guarding the treasure of maintainability."

## Architectural Components

### 1. The Cartographer (World Generation)
*   **Tools:** `filesystem`, `shell` (cloc, scc)
*   **Action:** Maps the codebase directory structure to a "World Map".
    *   `src/legacy` -> "The Ruins of the Ancients" (High Danger/Debt)
    *   `src/core` -> "The Citadel" (High Value/Defense)
    *   `tests/` -> "The Training Grounds"
*   **Metric:** "Danger Level" = Cyclomatic Complexity + Age of File + TODO count.

### 2. The Encounter Engine (Combat)
*   **Tools:** `shell` (git diff), `memory`
*   **Mechanism:**
    *   **Bosses:** Large, complex files are assigned HP (1 line of code = 1 HP).
    *   **Combat:** When a PR reduces line count or complexity, damage is dealt.
    *   **Status Effects:**
        *   *Rot:* A file untouched for 6 months gains a "Rot" debuff (increasing difficulty).
        *   *Shielded:* A file with 0% test coverage takes 50% less damage from refactoring (requires "Cleric" intervention first).

### 3. The Scribe (Persistence & Narrative)
*   **Tools:** `memory`, `filesystem`
*   **Artifacts:**
    *   `ADVENTURE_LOG.md`: A narrative history of the project ("On Nov 25, Paladin Alice struck the AuthModule for 500 damage!").
    *   `HEROES.json`: Persistent stats for every contributor.
*   **Memory Graph:**
    *   Stores the "Soul" of the code (historical complexity stats) to prevent "Boss Regeneration" (someone undoing a refactor).

### 4. The Class System (Role Detection)
*   **Tools:** `grep`, `memory`
*   **Logic:**
    *   **Warrior:** High volume of feature code additions.
    *   **Rogue:** High volume of deletions (Code cleanup).
    *   **Cleric:** High volume of test file edits.
    *   **Wizard:** High volume of configuration/build script edits.
    *   **Bard:** High volume of Markdown/Documentation edits.
*   **Action:** The agent detects playstyles and auto-assigns classes, unlocking class-specific "Quests" (e.g., "Cleric Quest: Heal the `UserFactory` by adding 5 tests").

## Loop Structure (Autonomous Batch)
1.  **Initialize:** Load `dungeon_state.json` (Memory Graph).
2.  **Scan:** Analyze current codebase metrics (Line counts, complexity).
3.  **Diff:** Compare with previous state.
    *   *If Lines Reduced:* Calculate Damage.
    *   *If Tests Added:* Calculate Healing/Buffs.
    *   *If Bugs Fixed:* Calculate XP.
4.  **Narrate:** Generate the "Turn Report".
5.  **Update:** Write changes to `ADVENTURE_LOG.md` and `HEROES.md`.
6.  **Quest Gen:** If a Boss dies, spawn new "Loot" (a Badge) and generate new Side Quests.

## Failure Modes & Recovery
*   **Metric Gaming:** Developers writing verbose code to get "Warrior" XP.
    *   *Mitigation:* The "Dungeon Master" applies a "Bloat Penalty" (XP penalty for high churn with low value). Requires `shell` access to `git-efforts` or similar analysis.
*   **Noise:** Too many updates in the log.
    *   *Mitigation:* Only report "Significant Events" (>100 damage or Level Ups).

## Human Touchpoints
*   **The Tavern (PR Comments):** The agent comments on PRs with RPG flavor text: *"Thy blade is sharp! You have severed 40 lines of spaghetti code from the Beast."*
*   **Command Line:** `dm status` (via shell alias) to see current quests.

## Implementation Roadmap
1.  **Level 1 (MVP):** `grep` based TODO hunter that awards "Gold" in a Markdown file.
2.  **Level 5 (Beta):** "Boss Fight" mechanic tracking file sizes over time.
3.  **Level 10 (Live):** Full "Class System" and "Party Raids" (multi-user coordination).
