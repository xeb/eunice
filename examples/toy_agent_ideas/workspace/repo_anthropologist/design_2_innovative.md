# Design 2: The Cultural Ghostwriter

## Purpose
A "Style Transfer" agent for social interactions. It rewrites your PR descriptions, commit messages, and even code comments to mimic the specific voice and style of the repository's core maintainers, increasing the likelihood of acceptance (Socio-Technical Congruence).

## Loop Structure
1.  **Ethnographic Modeling:**
    *   Agent searches `web` for the top 5 contributors to the repo.
    *   It fetches their recent comments and commits.
    *   It builds a "Persona Vector" in `memory` (e.g., "Terse, technical, never uses emojis, always references the ticket number").

2.  **Drafting Phase:**
    *   User writes a rough draft of their PR description: "I fixed the login bug."
    *   Agent activates "Maintainer Persona" (e.g., Linus Torvalds mode vs. Dan Abramov mode).
    *   Agent rewrites the text: "auth: fix race condition in login flow. Closes #123. (See discussion in #119)."
    *   Agent uses `text-editor` to replace the user's draft in the PR template.

## Tool Usage
*   **Web:** Brave Search to find the "Voice" of the repo (issue threads, mailing lists).
*   **Memory:** Storing the "Style Models" (Persona entities).
*   **Text-Editor:** Directly modifying the `PULL_REQUEST_TEMPLATE.md` or the commit message file.
*   **Shell:** `git commit --amend` with the new message.

## Memory Architecture
*   **Entities:** `Persona`, `Phrase`, `StyleRule`.
*   **Observations:** "Maintainer X often uses the word 'nit' for small errors." "Maintainer Y requires 'Impact Analysis' section."

## Failure Modes
*   **Uncanny Valley:** The agent sounds like a mockery of the maintainer.
    *   *Recovery:* User edits the output to be more natural.
*   **Over-fitting:** Adopting a style that is too specific to a senior maintainer (e.g., acting like you have authority you don't).
    *   *Recovery:* The agent should have a "Newcomer" mode that is polite but culturally aligned, rather than mimicking the BDFL (Benevolent Dictator For Life).

## Human Touchpoints
*   **Approval:** The agent presents the rewritten text for Yes/No approval before saving.
