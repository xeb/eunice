# Design 2: The Resurrector (Reactive)

## Purpose
A "Self-Healing" agent that focuses on link rot. It does not archive everything proactively. Instead, it regularly checks the health of links. If a link dies (404/DNS error), it hunts for a replacement (Internet Archive, Google Cache, or a new URL if the page moved) and patches your file.

## Loop Structure
1. **Audit**: Periodically (e.g., weekly) scans all Markdown files for links using `grep`.
2. **Ping**: Sends HEAD requests via `fetch` to verify status.
3. **Rescue**: If status is 404/NxDomain:
    - Search `web` (Internet Archive API) for the closest snapshot.
    - Search `web` (Google/Brave) for the page title to see if it moved.
4. **Patch**: If a valid substitute is found:
    - Uses `text-editor` to replace `[Link](http://dead.url)` with `[Link (Archived)](http://web.archive.org/...)` or `[Link (Moved)](http://new.url)`.
5. **Report**: Generates a "Dead Link Report" Markdown file for manual review of unfixable links.

## Tool Usage
- **fetch**: Checking link health (HEAD requests).
- **web**: Searching for mirrors or moved content.
- **text-editor**: In-place replacement of broken links.
- **memory**: Caching health status (don't check same URL every hour).

## Memory Architecture
- **Entities**: `URL`
- **Observations**: "Last Alive Date", "Death Date", "Redirect Location".

## Failure Modes
- **False Positives**: Site blocks HEAD requests. -> Double check with GET or browser headers.
- **Bad Archives**: Internet Archive snapshot is a "Parked Domain" page. -> Analyze content length/keywords before accepting.

## Human Touchpoints
- **Approval**: Could require a "Pull Request" style approval file before modifying links.
- **Notification**: "I fixed 5 broken links in 'Project X.md' today."
