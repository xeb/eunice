# Design 2: The Simulationist (Innovative)

## Purpose
An active "Social Sandbox" that prevents faux pas by **simulating** community reaction to a user's drafted message *before* they post it. It acts as a "Cultural Linter."

## Loop Structure
1.  **Drafting:** User writes a draft email, PR description, or forum post in a watched file (e.g., `drafts/message.md`).
2.  **Context Loading:** Agent loads the "Norm Model" for the target community from Memory.
3.  **Simulation:**
    *   Agent compares the draft against "Taboo Patterns" (e.g., "Don't ask for ETAs").
    *   Agent checks for "Missing Shibboleths" (e.g., "Did you include the required reproduction steps?").
4.  **Feedback:** Agent annotates the draft file with comments:
    *   "⚠️ Risk: This tone is too informal for this mailing list."
    *   "✅ Suggestion: Add a 'Prior Art' section, as this community values thoroughness."
5.  **Refinement:** User edits; Agent re-scans.

## Tool Usage
*   **Filesystem:** Watches a specific directory for drafts.
*   **Memory:** High-fidelity graph of "Reaction Patterns" (Input -> Likely Reaction).
*   **Web:** Fetches similar *past* posts to show as examples ("See how this similar post was received...").

## Memory Architecture
*   **Pattern Matching:**
    *   Node: `Pattern: "Is this dead?"`
    *   Relation: `TRIGGERS_REACTION` -> `Reaction: Hostility`
*   **User Reputation:** Tracks the user's *own* standing in the community (if provided) to adjust risk tolerance.

## Failure Modes
*   **False Positives:** Flagging valid questions as "risky" because similar words were used in bad faith previously. *Recovery:* "Ignore Rule" command.
*   **Drift:** Community norms change; the model becomes outdated. *Recovery:* Periodic "Re-calibration" scans of new threads.

## Human Touchpoints
*   **The "Post" Decision:** The agent never posts; it only advises. The human has the final button press.
