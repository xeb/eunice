# Final Design: The Reality Anchor

## One-Line Summary
A background daemon that anchors static documentation to the evolving real world by detecting link rot, outdated versions, and appending dynamic context reports to opt-in files.

## Core Philosophy
**"Preserve the Past, Annotate the Present."**
The agent never rewrites the user's original notes (which may be historically significant). Instead, it frames them with non-destructive annotations:
1.  **Header:** Critical warnings (Broken links, Deprecated software).
2.  **Footer:** Dynamic context (Latest versions, Relevant news).

## Architecture

### 1. The Anchoring Loop (Autonomous)
*   **Frequency:** Weekly or On-Demand.
*   **Scope:** Recursive scan of the `notes/` directory.
*   **Steps:**
    1.  **Parsing:** Identify "Anchors" (URLs, Version Regex, Dates) and "Subscriptions" (Tags like `#anchor:ReactJS`).
    2.  **Verification:**
        *   Batch `fetch` requests for URL health.
        *   `web_brave_web_search` for "Current Stable Version of [Artifact]".
        *   `web_brave_news_search` for "Security Vulnerabilities [Artifact] 2025".
    3.  **Memory Sync:** Update the knowledge graph with `LastVerified` timestamps to prevent redundant web traffic.
    4.  **Annotation:**
        *   If `broken_links > 0`: Prepend `> [!CAUTION] 3 Links in this file are broken (checked 2025-11-25).`
        *   If `#anchor:topic` exists: Append/Update the **Reality Report** footer.

### 2. The Toolchain
*   **memory (The Anchor Chain):**
    *   *Purpose:* Caching verification results to be polite to servers and speed up local scans.
    *   *Schema:* `Entity(Type="URL", Status="404")`, `Entity(Type="Software", Name="Node.js", Version="22.0")`.
*   **filesystem (The Harbor):**
    *   Reads Markdown files.
    *   Uses `text-editor` to safely patch headers/footers without touching body text.
*   **web_brave (The Lookout):**
    *   Used for both "Fact Checking" (versions) and "Context Gathering" (news).
*   **grep (The Radar):**
    *   Fast pre-filtering of files to avoid reading terabytes of irrelevant data.

### 3. Example Reality Report (Footer)
```markdown
---
### ⚓ Reality Anchor Report (2025-11-25)
*   **Link Health:** 🟢 All Systems Go
*   **Software Status:**
    *   `React v16` detected in text → ⚠️ **Outdated** (Current: v19.0)
    *   `Node.js v14` detected in text → ⛔ **End-of-Life**
*   **Recent Context:**
    *   [News] React 19 introduces Compiler (2024-10-15)
```

## Failure Recovery
*   **Offline Mode:** If `web_brave` fails or is rate-limited, the agent skips verification and only reports on previously cached states from `memory`.
*   **Conflict:** Uses `text-editor` hash-based patching to ensure it doesn't overwrite user edits while appending reports.

## Implementation Roadmap
1.  **Phase 1 (The Link Checker):** Implement URL regex + `fetch` HEAD requests + Memory caching.
2.  **Phase 2 (The Version Scout):** Implement "Software Version" entity recognition + Search queries.
3.  **Phase 3 (The News Reporter):** Implement `#anchor` tag parsing and footer generation.
