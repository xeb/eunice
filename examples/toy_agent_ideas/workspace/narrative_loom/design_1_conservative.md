# Design 1: The Daily Diff (Conservative)

## Purpose
The **Daily Diff** agent focuses on precise, incremental tracking of specific ongoing news stories. Its primary goal is to filter out repetitive noise and highlight *only what changed* since the last update. It solves the problem of "news fatigue" where users see the same headlines repeatedly without understanding the actual progress of a story.

## Loop Structure
1.  **Wake Up:** Scheduled execution (e.g., every 6/12/24 hours).
2.  **Context Loading:** Reads the `tracked_topics.json` (active stories) and `last_state_summary.md` (what we knew yesterday).
3.  **Fetch:** Uses `web_brave_news_search` for each active topic.
4.  **Diff Analysis:** 
    *   Compares new articles against the `last_state_summary`.
    *   Identifies: New Entities, New Claims, Resolved Questions, Contradictions.
5.  **Update:**
    *   Updates `last_state_summary.md` with the new "Current Truth".
    *   Appends to `changelog.md` with a timestamped entry of the diff.
6.  **Report:** Generates a "Delta Briefing" for the user.

## Tool Usage
*   **web:** `web_brave_news_search` for gathering updates.
*   **filesystem:** 
    *   `read_file`/`write_file` for managing state JSONs and Markdown reports.
    *   No complex database; relies on flat files for simplicity and portability.
*   **memory:** Minimal usage, perhaps just to store user preferences or permanent entity blocks.

## Memory Architecture
*   **State-based:** The "Memory" is primarily the `last_state_summary.md` file. It acts as the "snapshot" of the world.
*   **Ephemeral:** It doesn't build a complex graph. It just needs to know "What was true yesterday?" to answer "What is new today?".

## Failure Modes
*   **Hallucinated Differences:** The LLM might misinterpret a rephrased sentence as a "new fact".
    *   *Recovery:* User feedback loop to mark updates as "False Positive".
*   **Missed Context:** If a story shifts terminology (e.g., "The bill" becomes "The Law"), it might lose track.
    *   *Recovery:* Periodic "Full Re-scan" of the topic.

## Human Touchpoints
*   **Topic Selection:** User manually adds topics to `tracked_topics.json`.
*   **Feedback:** User can flag "No update" reports to tune sensitivity.
