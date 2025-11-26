# Design 3: The Empathy Injector (Experimental)

## Purpose
To reduce the psychological distance between "Code" and "User" by simulating user reactions *during* the development process. Instead of reporting issues, the agent creates "Personas" that inhabit the codebase and "react" to changes in PRs.

## Loop Structure
1.  **Persona Generation:** Analyze `web` search results to build 5-10 persistent "Archetypes" (e.g., "Speedy Steve" who hates lag, "Boomer Bob" who struggles with UI changes). Store in `memory`.
2.  **Context Monitoring:** When a file is modified (detected via `filesystem` or git hook), the agent checks the "Sentiment History" of that file.
3.  **Simulated Reaction:**
    *   If `Login.tsx` is touched, and "Steve" has complained about login speed in the past, the agent posts a comment: *"Steve (Persona): Please make sure this doesn't slow down login again. I'm already frustrated."*
4.  **Test Enhancement:** The agent generates "User Story Tests" (text files) that narrate a user's struggle, derived from real reviews.

## Tool Usage
*   **web_brave_web_search**: Sourcing raw material for personas.
*   **memory_create_entities**: Building complex Persona nodes with "Traits" and "Pet Peeves".
*   **filesystem_edit_file**: Injecting "Persona Comments" into the top of files as JSDoc/Docstrings (e.g., `@user-pain-point This module causes 40% of our support tickets`).

## Memory Architecture
*   **Nodes**: `Persona`, `Trait` (e.g., "Low Bandwidth"), `UserStory`.
*   **Edges**: `HAS_TRAIT`, `CARES_ABOUT_FEATURE`.
*   **Persistence**: Personas evolve. If users stop complaining about speed, "Steve" becomes happier.

## Failure Modes
*   **Annoyance**: Developers finding the "Persona" comments patronizing or distracting.
    *   *Recovery:* Strict "Opt-in" per module. Only inject on "High Risk" files.
*   **Stereotyping**: Generative personas becoming caricatures.
    *   *Recovery:* Ground every persona trait in *actual* quoted feedback URLs.

## Human Touchpoints
*   **Collaboration**: Developers "talk" to the personas in PR comments (simulated).
*   **Config**: Humans define which personas are active.
