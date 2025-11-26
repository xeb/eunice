# Design 2: The Mental Twin

## Purpose
To autonomously track developer attention and intent by "shadowing" their activity, creating a persistent "Mental Model" in a graph database that can be queried or restored at any time without manual data entry.

## Core Loop (Daemon)
1.  **Observe:** Continuously tail `.bash_history` and file system events (fswatch).
2.  **Infer:**
    *   *Pattern:* Editing `*.test.js` + Failing Builds = "Debugging Mode".
    *   *Pattern:* Editing `*.md` + No Builds = "Documentation Mode".
3.  **Graph Update:**
    *   Create `Session` nodes.
    *   Link `File` entities to `Session` with `EDITED_IN` edges.
    *   Update `UserFocus` entity with current inferred topic.
4.  **Inactivity Trigger:** If idle > 15m, mark Session as "Suspended".
5.  **Resumption:** When activity resumes:
    *   Query Memory Graph for last "Suspended" session.
    *   Compare "World State" (Git) vs "Mental State" (Graph).
    *   *Proactive Alert:* "While you were gone, 3 commits touched the files you were editing."

## Tools
*   **memory:** Storing the complex graph of files, commands, and inferred intent.
*   **shell:** Monitoring processes and git.
*   **grep:** Scanning modified files for keywords to infer topic.

## Memory Architecture
*   **Graph Database:**
    *   **Nodes:** `Developer`, `Session`, `File`, `Topic`, `Command`.
    *   **Edges:** `FOCUSED_ON`, `MODIFIED`, `BROKE`, `FIXED`.
*   **Inference Engine:** A background logic loop that creates `Topic` nodes based on file content analysis.

## Failure Modes
*   **Misinterpretation:** Agent infers "Refactoring" when user is just "Browsing".
    *   *Recovery:* User can explicitly correct the graph: "No, I am Researching."
*   **Performance:** Continuous monitoring might lag the system.
    *   *Mitigation:* Polling intervals and efficient file watching.

## Human Touchpoints
*   **Implicit:** User just works.
*   **Notifications:** Agent sends async notifications/terminal echoes upon return.
