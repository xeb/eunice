# Design 1: The Research Sidebar (Conservative)

## Purpose
To provide non-intrusive, just-in-time research and context for writers without breaking their flow. It acts as a proactive librarian who places relevant books on your desk before you ask for them.

## Loop Structure
1. **Watch:** The agent monitors a specific directory for changes to Markdown (`.md`) files.
2. **Analyze:** When a file is modified, the agent reads the last N paragraphs.
3. **Extract:** It identifies key nouns, historical terms, or technical concepts (e.g., "The Battle of Hastings", "quantum entanglement", "Victorian fashion").
4. **Search:** It performs a `web_brave_web_search` for these terms.
5. **Curate:** It summarizes the top 3 results into bullet points.
6. **Update:** It appends these findings to a companion file (e.g., `chapter1_research.md`) or updates a "Sidecar" section at the bottom of the main file.

## Tool Usage
*   **filesystem:** `filesystem_read_text_file` to monitor content, `filesystem_write_file` to update the research sidecar.
*   **web:** `web_brave_web_search` to find facts, dates, and definitions.
*   **grep:** `grep_search` to check if terms have already been researched to avoid redundancy.

## Memory Architecture
*   **Filesystem-based:** The "memory" is simply the `_research.md` file. This ensures the user can read, edit, or delete the research as easily as their own writing. It requires no complex graph database.

## Failure Modes
*   **Distraction:** The agent might fetch irrelevant info. *Recovery:* User ignores the sidecar file.
*   **Hallucination:** Summaries might be inaccurate. *Recovery:* Agent provides raw URLs for verification.
*   **Over-activity:** Searching too often. *Mitigation:* Rate limiting (e.g., only search after 5 minutes of idle time or explicitly marked `// TODO: research` comments).

## Human Touchpoints
*   **Passive:** The user never interacts directly with the agent. They just see the `_research.md` file grow.
*   **Explicit Trigger:** The user can type `[[Query]]` in their text to force a specific search.
