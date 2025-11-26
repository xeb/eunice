# Design 3: The Inclusive Refactorer

## Purpose
A "Teaching Assistant" agent that not only finds and fixes issues but **educates** the developer on *why* the fix matters. It operates as a "Code Reviewer" that opens Pull Requests with detailed educational context, helping teams upskill in accessible design.

## Loop Structure
1.  **Surveillance:** Watches `filesystem` for changes to UI files.
2.  **Diagnosis:** Identifies accessibility patterns (like Design 1) but also looks for **semantic opportunities** (e.g., "This `div` looks like it acts as a button").
3.  **Research:** Uses `web` to find the specific WCAG Success Criterion and "Best Practice" patterns for the detected issue.
4.  **Remediation:** Uses `text-editor` to generate a patch.
    *   *Example:* changing `<div onClick={...}>` to `<button type="button" onClick={...}>`.
5.  **Documentation:** Generates a PR description that includes:
    *   **The Problem:** (e.g., "Divs are not keyboard accessible").
    *   **The Fix:** (The code change).
    *   **The Why:** (Link to WCAG 2.1.1 Keyboard).
    *   **The Lesson:** A brief snippet on how to avoid this in the future.

## Tool Usage
*   **text-editor:** Essential for applying granular patches without rewriting entire files.
*   **web:** Fetches educational content and standard examples.
*   **memory:** Tracks "Developer Skills". If a developer repeatedly makes the same error, the agent adjusts its tone to be more instructive or link to different resources.

## Memory Architecture
*   **Entities:** `Developer`, `Skill`, `ErrorPattern`.
*   **Relations:** `Developer HAS_LEARNED Pattern`, `Developer REPEATS Error`.
*   **Purpose:** To personalize the "coaching".

## Failure Modes
*   **Regression:** Automated fixes might break CSS styling (e.g., `button` has different default styles than `div`).
    *   *Mitigation:* The agent adds CSS comments or explicitly sets styles to `all: unset` when converting elements, or flags the change for visual review.
*   **Annoyance:** Too many educational messages can be patronizing.
    *   *Recovery:* Memory tracks "Dismissals" and reduces verbosity if the user ignores the advice.

## Human Touchpoints
*   **PR Review:** The primary interaction is reviewing the agent's PRs.
*   **Dialogue:** The user can comment on the PR to ask "Why?", and the agent (via Memory/Web) can reply with more info.
