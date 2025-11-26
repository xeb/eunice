# Design 2: The World Builder (Innovative)

## Purpose
To maintain internal consistency in fiction or complex technical writing. It acts as a "Continuity Editor," ensuring that characters don't change eye color and laws of physics remain constant.

## Loop Structure
1. **Ingest:** The agent parses the text sentence-by-sentence.
2. **Entity Recognition:** It uses an LLM to identify entities (People, Places, Objects) and their attributes (Color, Age, Location, Relation).
3. **Graph Update:** 
    *   If the entity is new, create a Node in Memory.
    *   If the entity exists, add a new Observation.
4. **Consistency Check:** 
    *   Compare the new observation with existing facts in the Memory Graph.
    *   If a contradiction is found (e.g., "Bob is 30" vs "Bob is 45"), flag it.
5. **Intervene:**
    *   Use `text-editor_edit_text_file_contents` to insert a non-breaking comment or warning near the text: `<!-- WARNING: Bob was 30 in Chapter 1 -->`.

## Tool Usage
*   **memory:** `memory_create_entities`, `memory_add_observations`, `memory_read_graph` to maintain the World Bible.
*   **text-editor:** To insert inline warnings/comments.
*   **web:** Optional, to check external consistency (e.g., "Is this travel time realistic for 1850s London?").

## Memory Architecture
*   **Graph-Native:** The core truth is stored in the Memory MCP graph.
*   **Exportable:** The agent can periodically dump the graph to a `world_bible.md` for the author to review.

## Failure Modes
*   **False Positives:** Flagging poetic metaphor as literal contradiction. *Recovery:* User marks fact as "metaphor" or deletes the comment.
*   **Graph Bloat:** Tracking too many insignificant objects (e.g., "a cup of coffee"). *Mitigation:* Filter for named entities or recurrent nouns only.

## Human Touchpoints
*   **Correction:** The user can edit the `world_bible.md` to correct the agent's understanding.
*   **Approval:** The agent essentially "submits issues" to the text.
