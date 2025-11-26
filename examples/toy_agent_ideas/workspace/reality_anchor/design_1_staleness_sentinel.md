# Design 1: The Staleness Sentinel

## Purpose
A conservative, high-precision agent designed to detect objective "rot" in documentation: broken links, deprecated software versions, and referenced dates that are far in the past. It acts as a "Janitor" for technical notes.

## Loop Structure
1.  **Scan Phase:**
    *   Iterate through user-specified directories using `filesystem_list`.
    *   Use `grep` to identify "Anchors":
        *   URLs (http/https)
        *   Version strings (vX.Y, "version X")
        *   Dates (YYYY-MM-DD, "Summer 202X")
2.  **Verification Phase:**
    *   For URLs: Use `fetch` (HEAD request) to check for 404/500 codes.
    *   For Versions: Use `web_brave_web_search` ("current version of [Software Name]") and parse snippets for numbers higher than the one found.
    *   For Dates: Compare against `date +%Y`. If > 2 years old, flag as "Potentially Stale".
3.  **Reporting Phase:**
    *   **Memory Update:** Store checked Anchors in `memory` (Entity: `Anchor`, Property: `last_checked`, `status`). Avoid re-checking recently verified items.
    *   **Annotation:** Use `text-editor` to append a non-intrusive metadata block or "Admonition" to the file header.
        *   *Example:* `> [!WARNING] 3 broken links and 1 deprecated version detected (2025-11-25).`

## Tool Usage
*   **filesystem:** Read files, list directories.
*   **grep:** Fast extraction of patterns.
*   **fetch:** Link validation.
*   **web_brave:** Version checking.
*   **memory:** Deduping checks (don't check google.com every time).
*   **text-editor:** Inserting warnings.

## Memory Architecture
*   **Nodes:** `File`, `URL`, `SoftwareArtifact`.
*   **Edges:** `File CONTAINS URL`, `SoftwareArtifact HAS_VERSION String`.
*   **Properties:** `last_verified_timestamp`, `http_status`, `latest_known_version`.

## Failure Modes
*   **False Positives:** "Python 2.7 is great" (historical statement) vs "Use Python 2.7" (instruction). The agent might flag historical facts as "outdated instructions".
*   **Rate Limits:** Checking 1000 links might trigger WAFs. *Mitigation:* Backoff + Memory caching.

## Human Touchpoints
*   **Passive:** User sees warnings in their editor.
*   **Active:** User runs a "Fix" command (e.g., "Update all versions") which is risky and requires confirmation. This design is primarily *read-only* (annotation).
