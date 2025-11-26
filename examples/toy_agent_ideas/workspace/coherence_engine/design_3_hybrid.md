# Design 3: The Consilience Engine (Hybrid)

## Purpose
To act as a "Socratic Mirror" for the codebase. Instead of just flagging errors, it facilitates a dialogue between the "Intent" (Docs/Comments) and the "Implementation" (Code), using the filesystem as a communication bus.

## Core Philosophy
"Inconsistency is an opportunity for clarification." The agent does not judge right/wrong but highlights *divergence*. It treats the codebase as a conversation that has stalled.

## Loop Structure
1.  **Observation**:
    *   Agent monitors the `git` log via `shell`.
    *   Identifies "Hotspots" where code changes frequently but docs/tests do not.
2.  **Hypothesis Generation**:
    *   Formulates a hypothesis: "The documentation for X is likely stale because code changed 5 times in the last month."
    *   Stores this hypothesis in `memory`.
3.  **Interrogation (The "Review.md" Interface)**:
    *   Instead of polluting code with comments, it creates a `reviews/YYYY-MM-DD-coherence.md` file.
    *   It populates this file with "Questions":
        *   "Q: I see you changed `calculateTax`. The docstring mentions 'VAT' but the code now references 'GST'. Is this intentional?"
4.  **Interaction**:
    *   Developer answers in the Markdown file (e.g., "A: Yes, we migrated to GST.").
    *   Agent parses the answer.
    *   Agent *automatically updates* the docstring if the answer confirms the drift (using `text-editor`).

## Tool Usage
*   **shell**: Run git commands to analyze churn and blame.
*   **filesystem**: Create and read the "Review" markdown files.
*   **text-editor**: Apply user-approved patches to documentation.
*   **memory**: Track which questions have been asked to avoid nagging.

## Memory Architecture
*   **Interaction Log**: Records "Interrogations" and "Resolutions".
*   **Staleness Index**: Maps files to a "Staleness Score" based on churn vs. doc updates.

## Failure Modes
*   **Ignored Questions**: Developer ignores the review file. Strategy: Agent archives the review after 7 days and marks the metadata as "Unverified".
*   **Hallucination**: Agent proposes a wrong doc update. Strategy: The "Review" file is just text; the agent only edits code after explicit "Yes" in the markdown.

## Human Touchpoints
*   **Async Chat**: The `reviews/` folder acts as an asynchronous chat interface.
*   **Approval**: User signals intent by typing "Yes" or checking a checkbox `[x]` in the markdown.
