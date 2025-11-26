# Design 3: The "Release Narrator" (Hybrid)

## Purpose
Combines the rigour of Code Analysis with the "Human Element" of narrative. It uses the "Semantic Impact Graph" to determine the *Version Number*, but uses "Conventional Commits" and *LLM summarization* (simulated via simple text templates or an external call if allowed) to write a human-readable "Story of the Release".

## Loop Structure
1.  **Hybrid Analysis:**
    *   Check Code Graph for *Version Validity* (Major/Minor/Patch).
    *   Check Commit Messages for *Context* (Why was this done?).
2.  **Conflict Detection:** If the Code Graph says "MAJOR" but the Human says "fix: typo", the agent halts and alerts: "Code change in `api.ts` breaks the interface, but you marked it as a fix. Please clarify."
3.  **Narrative Generation:** Groups changes by "Theme" (e.g., "Auth System Overhaul") rather than just listing commits.
4.  **Artifact Creation:** distinct artifacts:
    *   `CHANGELOG.md` (Human readable)
    *   `API_DIFF.json` (Machine readable breaking change report)

## Tool Usage
*   **Memory:** Stores "Themes" or "Epics" that cross multiple commits.
*   **Filesystem:** updates changelogs, reads code.
*   **Shell:** Git operations.

## Memory Architecture
*   **The "Release Context":** A temporary graph node representing the "Upcoming Release" that collects commits and observations until the `release` command is triggered.

## Failure Modes
*   **Context Hallucination:** If assuming an LLM, it might invent reasons. Without LLM, it relies on potentially bad commit messages for the "Why".

## Human Touchpoints
*   **Theme Selection:** User can group commits into "Features".
*   **Final Polish:** User edits the generated Changelog before push.
