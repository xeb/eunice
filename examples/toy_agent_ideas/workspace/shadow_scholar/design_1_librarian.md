# Design 1: The Librarian (Conservative)

## Purpose
To aggregate dispersed community knowledge (StackOverflow, GitHub Issues, Discord logs) into a centralized "Community Notes" section within the official documentation, without altering the core "official" text. This ensures safety and prevents the agent from introducing hallucinations into trusted specs.

## Loop Structure
1.  **Harvest:** Every 24 hours, query `web` (Brave/GitHub API) for new discussions tagged with the project name.
2.  **Filter:** Select items with high engagement (upvotes > 10) or specific keywords ("workaround", "bug", "deprecated").
3.  **Synthesize:** Use an LLM to summarize the discussion into a "Tip" or "Warning".
4.  **Publish:** Append these summaries to a `docs/COMMUNITY_KNOWLEDGE.md` file, categorized by topic.
5.  **Prune:** Re-verify links monthly; if a GitHub issue is closed/merged, remove the note.

## Tool Usage
*   `web_brave_web_search`: To find recent StackOverflow discussions and blog posts.
*   `fetch_fetch`: To retrieve raw text of GitHub issues/comments.
*   `filesystem_write_file`: To append to the knowledge file.
*   `grep_search`: To check if a topic is already covered in the official docs (to avoid redundancy).

## Memory Architecture
*   **Filesystem-based State:** Uses a simple JSON file (`.librarian_state.json`) to track `last_scan_timestamp` and `processed_url_hashes` to prevent duplicate processing.
*   **No Graph:** Relies on the structure of the Markdown output as the knowledge base.

## Failure Modes
*   **Link Rot:** Referenced URLs go dead. *Recovery:* Periodic "Link Checker" mode.
*   **Misinformation:** Highly voted StackOverflow answers might be outdated. *Mitigation:* Explicit timestamps on all generated notes ("As of 2024-11-25...").
*   **Noise:** Too many trivial updates. *Mitigation:* High threshold for inclusion (e.g., minimum 10 upvotes).

## Human Touchpoints
*   **Passive:** Humans simply read the generated file.
*   **Curator:** A human can add a `<!-- ignore -->` comment to specific sections of the knowledge file to prevent the agent from overwriting manual edits.
