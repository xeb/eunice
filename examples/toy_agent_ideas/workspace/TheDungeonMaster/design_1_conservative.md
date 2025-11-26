# Design 1: The Quest Board (Conservative)

## Purpose
To improve codebase hygiene by surfacing invisible work (TODOs, FIXMEs, missing tests) as visible, claimable "Quests" in a central Markdown dashboard.

## Loop Structure
1.  **Scan:** Periodically runs `grep` to find `TODO`, `FIXME`, `HACK`, `XXX` tags in the codebase.
2.  **Parse:** Extracts the comment text and the line number.
3.  **Assess:** Assigns a "Gold Reward" based on the age of the TODO (older = more gold) and the surrounding file complexity (estimated by file size).
4.  **Publish:** Regenerates `QUEST_BOARD.md` in the root directory.
    *   Sections: "New Bounties", "High Value Targets", "Daily Dailies" (e.g., "Write one test").
5.  **Verify:** When a PR closes a TODO, the agent detects the removal in the next scan and records the "payout" to a `LEADERBOARD.md`.

## Tool Usage
*   `grep`: Finding patterns.
*   `filesystem`: Reading files to estimate complexity; writing `QUEST_BOARD.md`.
*   `shell`: Running `git blame` to find the age of a TODO.

## Memory Architecture
*   **Stateless/Low-State:** Mostly relies on the file system state.
*   **Leaderboard:** A simple JSON file `dungeon/leaderboard.json` tracks user scores.

## Failure Modes
*   **Duplicate Quests:** If code moves, the TODO moves. Agent needs to track by content hash, not just line number.
*   **Gaming the System:** Devs adding TODOs just to delete them. (Mitigated by human code review).

## Human Touchpoints
*   **Claiming:** Humans edit `QUEST_BOARD.md` to add their name to a quest (e.g., `- [x] Fix memory leak @username`).
