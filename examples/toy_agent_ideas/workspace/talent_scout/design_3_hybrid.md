# Design 3: The Talent Graph (Network Weaver)

## Purpose
To leverage existing social connections within the engineering team to find warm introductions to talent, rather than cold emailing.

## Loop Structure
1.  **Interaction Mining:** Scans the *team's* public activity (if provided) or local git history to see who *current* employees have interacted with (e.g., commented on PRs, replied to issues) on external repos.
2.  **Graph Expansion:** Builds a "Second-Degree Network" graph. (Employee A knows External Dev B).
3.  **Skill Overlay:** Maps the external devs to the libraries they maintain.
4.  **Opportunity Spotting:** When a new hiring requirement arises (e.g., "Need Rust Expert"), it queries the graph: "Which external Rust experts do we already know?"
5.  **Briefing:** Generates a "Introduction Request" for the internal employee: "Hey [Employee], you chatted with [Target] on [Repo] last year. Can you reach out?"

## Tool Usage
*   **web:** Search specific interaction histories (comments, issues).
*   **memory:** Deep graph of `Person` -> `Interaction` -> `Person`.
*   **filesystem:** Read local git config to identify "Internal Employees".

## Memory Architecture
*   **Nodes:** `InternalUser`, `ExternalDev`, `InteractionEvent`.
*   **Edges:** `COLLABORATED_WITH`, `REVIEWED_CODE_OF`.

## Failure Modes
*   **Privacy:** Inadvertently scraping private interactions. *Mitigation:* Only scan public repo interactions.
*   **Stale Data:** Interactions from 5 years ago might be irrelevant.

## Human Touchpoints
*   **Trigger:** User defines a "Hiring Goal".
*   **Action:** User acts on the "Warm Intro" suggestion.
