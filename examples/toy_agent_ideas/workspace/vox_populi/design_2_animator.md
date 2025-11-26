# Design 2: The Issue Animator (Innovative)

## Purpose
To actively inject the "Voice of the Customer" into the developer's daily workflow (GitHub Issues/Jira) rather than hiding it in a separate report. This agent ensures that no "Critical" bug sits stale without fresh evidence of user pain.

## Loop Structure
1.  **Monitor:** Continuous background loop checking `web_brave_web_search` for new complaints.
2.  **Cluster:** Group complaints by semantic similarity (using Memory Graph).
3.  **Triage:**
    *   *If cluster maps to existing Issue:* Comment on the issue: "Update: 5 more users reported this today. Key quote: 'It's unusable'." -> Bump priority.
    *   *If cluster is new & high severity:* Draft a new Issue: "[Auto-Vox] Potential Regression in Checkout Flow (12 reports)".
4.  **Enrichment:** Use `web_brave_image_search` to find if users posted screenshots of the error, and attach links to the issue.

## Tool Usage
*   **web_brave_web_search**: Real-time feedback monitoring.
*   **memory_search_nodes**: finding existing Issues (mirrored in graph) that match the feedback topic.
*   **filesystem_write_file**: Creating "Draft Issue" markdown files in a specific `issues/inbox/` folder (which a CI job could sync to GitHub).

## Memory Architecture
*   **Nodes**: `Issue` (id, title, status), `UserComplaint` (text, url), `SentimentCluster` (topic).
*   **Edges**: `EvidenceFor` (Complaint -> Issue), `IsDuplicateOf`.
*   **Persistence**: Keeps track of which complaints have *already* been reported to avoid spamming the issue tracker.

## Failure Modes
*   **Spamming**: Agent comments on an issue 100 times for a viral tweet.
    *   *Recovery:* Rate limiting (max 1 comment per issue per day) and summarization ("+50 others reported this").
*   **Misclassification**: Creating a bug report for a feature request.
    *   *Recovery:* Human triage step. The agent creates "Drafts" first.

## Human Touchpoints
*   **Gatekeeper**: Humans review "Draft Issues" before they become real tickets.
*   **Consumer**: Developers see the agent's comments on existing tickets, providing motivation and context.
