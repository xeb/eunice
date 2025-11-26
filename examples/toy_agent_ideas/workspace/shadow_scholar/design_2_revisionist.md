# Design 2: The Revisionist (Experimental)

## Purpose
To actively combat "Documentation Drift" by treating the documentation as a living organism that *must* match reality. If the community consensus says "Function X is broken, use Function Y", this agent proactively edits the documentation for Function X to reflect this reality.

## Loop Structure
1.  **Monitor:** Continuous polling of issue trackers.
2.  **Conflict Detection:** When a high-confidence claim ("Docs are wrong", "Deprecated") is found, compare it against the text of the official docs using semantic search.
3.  **Localization:** Use `grep` and file reading to find the exact line numbers in the documentation source.
4.  **Mutation:** Generate a patch that updates the documentation.
5.  **Verification:** Run a "Tone Check" to ensure the edit sounds like official documentation, not a forum post.
6.  **Submission:** Create a Pull Request (simulated via file edit + branch creation command).

## Tool Usage
*   `memory_search_nodes`: To find which doc files correspond to which concepts (Concept -> File mapping).
*   `text-editor_edit_text_file_contents`: To apply surgical patches to documentation files.
*   `shell_execute_command`: To run git commands (branch, commit).
*   `web_brave_web_search`: To validate the claim against multiple sources (triangulation).

## Memory Architecture
*   **Truth Graph:** A `memory` graph that links **Concepts** (e.g., "Authentication") to **Assertions** (e.g., "Requires header X") and **Sources** (e.g., "Docs line 50", "Issue #402").
*   **Conflict Resolution:** If Source A (Docs) contradicts Source B (Community) and B has higher confidence/recency, B wins.

## Failure Modes
*   **Vandalism:** Malicious actors gaming the upvote system to trick the agent. *Recovery:* Whitelist of trusted community members/maintainers.
*   **Context Loss:** Deleting a "wrong" paragraph that was actually correct for a specific edge case. *Recovery:* "Conservative Delete" mode—wrap in `> [!WARNING]` block instead of deleting.

## Human Touchpoints
*   **Gatekeeper:** The agent produces PRs. Humans *must* merge them. The agent never pushes directly to main.
