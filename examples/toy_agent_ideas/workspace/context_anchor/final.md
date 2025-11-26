# Agent Design: The Context Anchor

## Executive Summary
**The Context Anchor** is an autonomous "Knowledge Dependency Manager" that eliminates link rot in software projects. Just as `npm` or `cargo` lock down code dependencies to specific versions to ensure reproducibility, The Context Anchor locks down *knowledge dependencies* (URLs in comments, documentation, and design specs) by creating local, immutable mirrors of referenced content.

## Core Philosophy
"If you link to it, you depend on it. If you depend on it, you must own a copy."

## Architectural Components

### 1. The Crawler (Discovery)
*   **Tools:** `grep`, `filesystem`
*   **Action:** Recursively scans the codebase for patterns matching `https?://`.
*   **Filter:** Ignores `localhost`, internal IPs, and domains listed in `.knowledgeignore`.
*   **Output:** A list of "Knowledge Dependencies".

### 2. The Archivist (Acquisition)
*   **Tools:** `fetch`, `shell` (calling `pandoc` or `single-file`), `web` (Wayback Machine)
*   **Action:**
    1.  **Check Liveness:** Pings the URL.
    2.  **Fetch:** Downloads the content.
    3.  **Sanitize:** Converts complex HTML to a simplified, single-file format (PDF, MHTML, or Markdown) to strip tracking scripts and ads.
    4.  **Fallback:** If the live link is dead (404), it queries the Wayback Machine API for the snapshot closest to the *commit date* of the line of code containing the link.
    5.  **Store:** Saves the artifact to `docs/external_assets/<domain>/<hash>.<ext>`.

### 3. The Locker (Persistence)
*   **Tools:** `filesystem`
*   **Artifact:** `knowledge-lock.json`
    ```json
    {
      "dependencies": {
        "https://react.dev/learn/preserving-and-resetting-state": {
          "status": "live",
          "local_path": "docs/external_assets/react.dev/a1b2c3d4.md",
          "last_checked": "2025-11-25",
          "hash": "sha256:...",
          "references": [
            "src/components/Form.js:42"
          ]
        }
      }
    }
    ```

### 4. The Editor (Intervention)
*   **Tools:** `text-editor`
*   **Action:**
    *   **Passive Mode:** Does nothing to the code.
    *   **Active Mode:** Rewrites the link in the source file to point to the local mirror (e.g., `[React State](../docs/external_assets/...)`).
    *   **Hybrid Mode (Recommended):** Appends a "Archive" link next to the original: `[React State](https://...) ([Mirror](../docs/...))`.

## Loop Structure (Autonomous Batch)
1.  **Initialize:** Load `knowledge-lock.json` and `.knowledgeignore`.
2.  **Scan:** Run `grep` across the project to find current links.
3.  **Diff:** Identify new links vs. locked links.
4.  **Process New:** Fetch, Sanitize, Store, Lock.
5.  **Audit Existing:** (Weekly) Check if locked links are still live. If they die, flag them in the lockfile but keep the local mirror (the "Anchor" holds).
6.  **Report:** Generate a Markdown report in `docs/knowledge_health.md`.

## Failure Modes & Recovery
*   **Dynamic Content:** Some sites (SPAs) return empty HTML via `fetch`.
    *   *Recovery:* Detect file size < 1KB. Retry with a headless browser tool (if available in shell) or flag for human manual download.
*   **Copyright/Legal:** Mirroring full content might violate terms.
    *   *Mitigation:* The agent defaults to "Private Archival" (files not committed to public git, but stored in a separate submodule or local-only folder).
*   **Infinite Loops:** Crawler might get stuck on calendar generators.
    *   *Mitigation:* Strict URL pattern matching and `.knowledgeignore`.

## Human Interaction
*   **Pull Request:** The agent runs in CI. If it finds new links, it adds the mirrored assets and the lockfile update to the PR.
*   **Approval:** The human approves the "knowledge vendoring" just like they approve a package.json change.

## Implementation Roadmap
1.  **MVP:** Python script using `grep` and `requests` to build the lockfile.
2.  **Phase 2:** Integration with <p>{“id”:15,“jsonrpc”:“2.0”,“method”:“tools/call”,“params”:{“arguments”:{“command”:“echo "test" &gt; workspace/test_shell.txt”},“name”:“execute_command”}} {“id”:16,“jsonrpc”:“2.0”,“method”:“tools/call”,“params”:{“arguments”:{“command”:“date”},“name”:“execute_command”}}</p> for clean Markdown conversion.
3.  **Phase 3:** Wayback Machine integration for "Time Travel Debugging" of broken links.
