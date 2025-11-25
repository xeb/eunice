# Design 1: The Librarian (Link Rot Repair)

## Purpose
To automatically detect and repair broken links in documentation and code comments by replacing them with historical snapshots from the Internet Archive (Wayback Machine). This ensures that "reference rot" never leaves a project with dead ends.

## Loop Structure
1.  **Scan:** Use `grep` to find all HTTP/HTTPS links in `.md`, `.py`, `.js`, etc.
2.  **Verify:** Use `fetch` (HEAD request) to check if the link is live (200 OK).
3.  **Search:** If 404/Timeout:
    *   Query the Wayback Machine API for the closest snapshot to the date the link was introduced (using git blame via `shell` to find the date).
4.  **Repair:**
    *   Use `text-editor` to replace the broken URL with the archive.org URL.
    *   Append a small marker (e.g., `[Archived]`) to indicate the change.
5.  **Report:** Generate a summary of "Rescued Links".

## Tool Usage
*   **grep:** `grep_search` with regex `https?://[^\s)]+` to extract URLs.
*   **fetch:** To ping URLs for liveness.
*   **web:** `brave_web_search` or direct API calls to find archived versions if the direct mapping fails.
*   **shell:** `git blame` to find when the link was added (crucial for finding the *right* version of the page).
*   **text-editor:** To surgically replace URLs without touching other code.

## Memory Architecture
*   **Stateless:** Relies on the state of the web (Internet Archive).
*   **Cache:** Uses a temporary JSON file to track checked URLs so it doesn't DDoS sites during a single run.

## Failure Modes
*   **Archive Missing:** If the page was never archived, the agent tags it as `[DEAD LINK]` and leaves a TODO for a human.
*   **False Positives:** Some sites return 200 OK even for soft 404s. The agent might miss these without content analysis.

## Human Touchpoints
*   **Review PR:** The agent submits a Pull Request with the URL updates. It never commits directly to main.
