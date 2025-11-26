# Design 3: The Guild Manager (Social/Hybrid)

## Purpose
To foster team collaboration and mentorship through a "Guild" metaphor, creating social pressure and rewards for non-feature work (reviews, mentoring, docs).

## Loop Structure
1.  **Party Formation:**
    *   Agent analyzes `git` history to find frequent co-editors.
    *   Suggests "Parties" for upcoming large features.
2.  **Raid Coordination:**
    *   Identifies a "Raid Target" (e.g., "Migrate to TypeScript").
    *   Tracks collective progress towards the goal (Shared Progress Bar).
    *   Distributes "Loot" (Badges) only if the *team* succeeds.
3.  **Mentorship Quests:**
    *   Detects a Junior Dev (low commit history in a module).
    *   Detects a Senior Dev (high history).
    *   Generates a "Side Quest": "Senior must review Junior's next 3 PRs."
    *   Reward: "Mentorship Badge" for Senior, "Level Up" for Junior.

## Tool Usage
*   `memory`: Social Graph (Who works with whom? Who knows what?).
*   `grep`: analyzing `CODEOWNERS` and git logs.
*   `web`: Fetching JIRA/Linear ticket statuses (optional).

## Memory Architecture
*   **Social Graph:** Nodes are Users, Edges are "REVIEWED", "PAIRED_WITH", "MENTORED".

## Failure Modes
*   **Exclusion:** New team members feeling left out of "Parties".
*   **Misinterpretation:** Identifying a "Junior" dev incorrectly.

## Human Touchpoints
*   **The Guild Hall:** A dedicated `GUILD.md` file where active parties and raids are listed.
