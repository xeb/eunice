# Design 2: The Socratic Ghost (Innovative)

## Purpose
To actively extract tacit knowledge by "interviewing" developers in real-time when they modify complex or legacy code. It acts as a curious junior engineer who asks "Why?" at the right moment.

## Loop Structure
**Trigger:** File Watcher / Git Hook (Pre-Push).

1.  **Change Detection:**
    *   Monitor changed files.
    *   Calculate "Surprise Factor": Is a junior dev touching a core system? Is a "stable" file (no edits > 1 year) being modified? Is the diff complex (high cyclomatic complexity increase)?
2.  **Socratic Querying:**
    *   If Surprise Factor > Threshold, the agent generates a specific question.
    *   *Prompt:* "I noticed you modified `LegacyBilling.ts`. This file handles the critical X logic. What was the tricky part of this change? Are there side effects we should watch for?"
    *   Delivery: CLI prompt (if local) or PR Comment (if CI).
3.  **Knowledge Graph Update:**
    *   Capture the developer's response.
    *   **Memory Entities:** `Developer`, `File`, `Concept`, `Risk`.
    *   **Memory Relations:** `Developer KNOWS Concept`, `File IMPLEMENTS Concept`, `Change INTRODUCED Risk`.
    *   **Observation:** "Dev X explained that `LegacyBilling` assumes Y about the database."
4.  **Documentation Synthesis:**
    *   Periodically, the agent reads the graph and updates a `KNOWLEDGE.md` or appends to the file's docstring using `text-editor`.

## Tool Usage
*   **memory:** Storing the graph of who knows what and the captured "micro-docs" from the interview.
*   **text-editor:** Injecting the captured knowledge back into the source code as comments.
*   **shell:** Git hooks, diff analysis.

## Memory Architecture
*   **Graph Database:** The core value is the relational graph.
    *   Nodes: Users, Files, Concepts (e.g., "Auth", "Payment").
    *   Edges: "Modified", "Explained", "ExpertIn".
    *   This allows queries like: "Who is the expert in Payment?" -> Returns User X because they have answered 5 questions about it.

## Failure Modes
*   **Annoyance:** Developers might find the agent intrusive. *Mitigation:* strict "cooldown" periods (don't ask more than once per day) and high thresholds for "Surprise".
*   **Garbage Input:** Devs typing "fixed bug" or "idk". *Mitigation:* Sentiment analysis on the answer; if low quality, discard or flag for peer review.

## Human Touchpoints
*   **The Interview:** The primary interaction.
*   **Graph Gardening:** Seniors occasionally review the "Concept" nodes to merge duplicates.
