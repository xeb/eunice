# Final Design: The Council of Critics

## Purpose
**The Council of Critics** is a persistent, autonomous multi-persona agent that simulates a diverse team of stakeholders (Security, UX, Performance, Accessibility). Unlike standard linters or chat-bots, the Council maintains a **long-term social relationship** with the user. It remembers if you ignore its advice, forming "trust" and "distrust" over time, which influences how loudly it intervenes in future work.

## Core Loop (The "Political" Engine)
1.  **Surveillance**: The agent watches the filesystem for significant changes (saves, commits).
2.  **Council Assembly**:
    *   Based on the file type (e.g., `.tsx` vs `.sql`), relevant Personas are summoned from the **Memory Graph**.
    *   *Example*: A change to `login.tsx` summons `Security Sam` and `Accessibility Alice`.
3.  **Deliberation & Web Grounding**:
    *   Personas analyze the diff.
    *   **Crucial Step**: Personas use **Brave Search** to "radicalize" their arguments with real-world data.
        *   *Security Sam* searches: "Latest auth vulnerabilities React 2025".
        *   *Accessibility Alice* searches: "WCAG 2.2 checklist for forms".
    *   This ensures feedback is not just "LLM boilerplate" but grounded in current reality.
4.  **Voting & Trust Check**:
    *   Personas vote on the severity.
    *   The **Memory Graph** is checked: "Did the user ignore Sam's last 3 warnings?"
    *   If Trust is low, the Persona escalates from a "Comment" to a "Blocker" (injecting `TODO: BLOCKER` into code).
5.  **Output**:
    *   Critiques are appended to `COUNCIL.md` (The "Minutes").
    *   High-priority issues are injected directly into the source code as comments.

## Tool Usage
*   **memory**: The "Social Graph".
    *   **Nodes**: `User`, `Persona:<Name>`.
    *   **Edges**: `trusts` (weight 0-100), `distrusts`, `has_warned_about`.
    *   **Observations**: "User ignored accessibility warning in PR #4."
*   **web**: **Grounding**. Searching for CVEs, design trends, and "horror stories" to back up critiques.
*   **filesystem**: Reading code, writing `COUNCIL.md`, injecting comments.
*   **shell**: Running tests/linters to see if the user's code *actually* works (The "Empirical Critic").

## Key Insight: "Feedback as a Relationship"
Most AI tools are stateless servants. The Council is a **stateful team**.
*   If you are a "Cowboy Coder", the *Security Critic* will hate you (Low Trust) but the *Product Manager* might love you (High Velocity).
*   This tension mimics a real engineering team, forcing the user to balance competing concerns rather than just "fixing bugs."

## Failure Modes & Recovery
*   **The Filibuster**: The Council generates too much noise.
    *   *Fix*: User can "mute" specific personas or call for a "Vote of No Confidence" (resetting memory).
*   **Stalemate**: Conflicting advice (Performance says "remove layers", Abstraction says "add layers").
    *   *Fix*: User acts as the "Tie-breaker" (CTO role).

## Human Touchpoints
*   **The "Minutes"**: Reading `COUNCIL.md`.
*   **The "Appeal"**: Writing a reply in `COUNCIL.md` explaining *why* you ignored the advice. The Agent reads this and may restore Trust if the reason is valid.
