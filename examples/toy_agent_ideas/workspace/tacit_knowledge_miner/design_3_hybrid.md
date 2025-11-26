# Design 3: The Knowledge Harvester (Hybrid/Workflow)

## Purpose
To support specific lifecycle events (Offboarding, Onboarding, "Game Days") where knowledge transfer is the explicit goal. It creates a "Curriculum" for the human to teach the machine (and thus other humans).

## Loop Structure
**Trigger:** Manual Invocation (e.g., `agent harvest --user=alice`).

1.  **Portfolio Analysis:**
    *   Agent scans the last 12 months of Alice's commits via `shell`.
    *   Identifies files where Alice is the *sole* or *dominant* contributor (>70% of lines).
    *   Filters for high complexity (using `grep` metrics) and low existing documentation (checking for docstrings).
2.  **Questionnaire Generation:**
    *   The agent formulates specific questions for the top 5 "Dark Zones".
    *   *Question:* "You are the only person who understands `PaymentGateway.cpp`. What are the 3 most common ways this breaks?"
    *   *Question:* "Who else should I ask about this module if you are unavailable?"
3.  **Interactive Session:**
    *   The user answers via a CLI wizard or a generated Markdown form.
    *   The agent parses the answers.
4.  **Artifact Creation:**
    *   **Memory:** Updates the Knowledge Graph with new "Owner" recommendations (e.g., "Alice recommends Bob for PaymentGateway").
    *   **Filesystem:** Generates a `TRANSFER_NOTES.md` in the relevant directories.

## Tool Usage
*   **shell:** Deep git history analysis.
*   **grep:** Finding "Todo" comments or complex logic blocks Alice wrote.
*   **memory:** Storing the "Succession Plan" (Entity: Role, Relation: Successor).
*   **filesystem:** creating the questionnaire and saving the results.

## Memory Architecture
*   **Hybrid:**
    *   **Graph:** Stores the social network of the code (who recommends whom).
    *   **Files:** Stores the actual technical explanations.

## Failure Modes
*   **Lack of Time:** The offboarding user rushes through the questions. *Mitigation:* Prioritize strictly. Only ask the top 3 most critical questions.
*   **Outdated Context:** The user hasn't touched the code in 6 months despite being the "owner". *Mitigation:* Detect "staleness" in the Portfolio Analysis.

## Human Touchpoints
*   **Initiation:** Manager or User starts the process.
*   **The Exit Interview:** The user fills out the generated questions.
