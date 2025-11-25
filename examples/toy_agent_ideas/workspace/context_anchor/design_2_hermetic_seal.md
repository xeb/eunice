# Design 2: The Hermetic Seal (Local Mirroring)

## Purpose
To create a fully self-contained project where every external reference is mirrored locally. This protects against the internet going down, domains expiring, or content changing. It treats documentation dependencies with the same rigor as code dependencies (vendoring).

## Loop Structure
1.  **Ingest:** scan codebase for URLs.
2.  **Download:** For every *new* URL:
    *   Use `fetch` or a headless browser via `shell` to download a simplified HTML/PDF version of the content.
    *   Save it to `docs/external_assets/<domain>/<hash>.html`.
3.  **Lock:** Update `knowledge-lock.json`:
    *   `"original_url": "https://..."`
    *   `"local_path": "docs/external_assets/..."`
    *   `"fetch_date": "2025-11-25"`
    *   `"hash": "sha256:..."`
4.  **Rewrite (Optional):**
    *   **Mode A (Sidecar):** Keep original links, but add a tooltip/icon `[Local Mirror]` next to them.
    *   **Mode B (Hardened):** Rewrite links to point to the local file, put original link in footer/comment.

## Tool Usage
*   **filesystem:** Heavily used to organize the `external_assets` directory structure.
*   **fetch:** To retrieve content.
*   **shell:** Can invoke tools like `pandoc` or `single-file-cli` to convert web pages into clean, single-file HTML or Markdown.
*   **grep:** To find links.

## Memory Architecture
*   **Filesystem-based Persistence:** The `knowledge-lock.json` is the source of truth. It can be committed to git.
*   **Deduplication:** Hashing content ensures we don't store 50 copies of the same React documentation page.

## Failure Modes
*   **Repo Bloat:** Storing PDFs and HTML can make the git repo huge. (Mitigation: Use `git-lfs` or store mirrors in an S3 bucket and only link to them).
*   **Dynamic Content:** SPAs (Single Page Apps) often fail to "fetch" correctly with simple tools.

## Human Touchpoints
*   **Allowlist:** Humans might need to ignore certain domains (e.g., `localhost`, `google.com`) via `.knowledgeignore`.
