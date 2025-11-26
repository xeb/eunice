# Design 1: The Auditor (Conservative)

## Purpose
The "Auditor" is a passive analysis agent designed to visualize the "weight" of software dependencies relative to their utility. It answers the question: *"Why is my build 50MB?"* by calculating a "Bloat Ratio" (Installed Size / Frequency of Use).

## Loop Structure
1.  **Scan:** Read manifest files (`package.json`, `requirements.txt`) to list direct dependencies.
2.  **Measure:**
    *   Use `shell` to run `du -sh` on dependency folders (e.g., `node_modules/library`).
    *   Use `web` to fetch metadata (Bundlephobia API, PyPI stats) for "Impact Cost".
3.  **Trace:**
    *   Use `grep` to count import statements across the codebase.
    *   Calculate usage frequency.
4.  **Report:** Generate a Markdown report highlighting "High Cost / Low Value" dependencies (e.g., a 2MB library used in only 1 file).

## Tool Usage
*   **filesystem:** Read manifests, write reports.
*   **shell:** Execute `npm list`, `du`, `pip freeze`.
*   **grep:** Count occurrences of library names in source files.
*   **web:** Search for "alternative lightweight libraries" for the worst offenders.

## Memory Architecture
*   **Stateless:** The agent runs as a one-off job. It does not maintain a persistent graph.

## Failure Modes
*   **Dynamic Imports:** `grep` might miss dynamic imports (`require(var_name)`), leading to false "unused" flags.
*   **Peer Dependencies:** Removing a "low usage" lib might break another lib that depends on it.

## Human Touchpoints
*   **Read-Only:** The agent never modifies code. It only produces artifacts (HTML/MD reports) for the human to review.
