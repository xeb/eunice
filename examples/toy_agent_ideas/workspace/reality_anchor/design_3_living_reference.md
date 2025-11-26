# Design 3: The Living Reference

## Purpose
A hybrid agent that doesn't just "check" facts but "enriches" them. It turns static notes into a dynamic dashboard by appending a "Live Context" footer to documents. It respects the original text as "Sacred History" and adds a "Modern Layer" on top.

## Loop Structure
1.  **Topic Modeling:**
    *   Scan file for capitalized phrases and frequency (using `grep`/`shell`). Identify the "Subject" of the note (e.g., "Kubernetes Deployment").
2.  **Context Fetching:**
    *   Query `web_brave_news_search` or `web_brave_web_search` for "Kubernetes Deployment best practices 2025".
    *   Fetch "Latest Release" info.
3.  **Synthesis:**
    *   Construct a Markdown footer:
        ```markdown
        ---
        ## 🟢 Reality Anchor Report (Generated 2025-11-25)
        *   **Latest Version:** v1.32 (detected v1.15 in text)
        *   **Trending Issues:** "CrashLoopBackOff in v1.32"
        *   **Related News:** "Kubernetes drops Docker shim"
        ```
4.  **Update:**
    *   Use `text-editor` to replace/update the existing footer (identified by a hash or specific header) without touching the body content.

## Tool Usage
*   **filesystem:** Read/Write.
*   **web_brave (News/Search):** Fetch dynamic context.
*   **memory:** track "Subscription" topics (what this note is *about*).

## Memory Architecture
*   **Nodes:** `Topic`, `File`.
*   **Edges:** `File SUBSCRIBES_TO Topic`.
*   **Properties:** `last_refresh`, `cached_summary`.

## Failure Modes
*   **Distraction:** Adding too much noise to simple notes.
*   **Irrelevance:** Fetching "Java Island" news for a "Java Programming" note.

## Human Touchpoints
*   **Configuration:** User tags files with `#anchor:topic` to explicitly opt-in to updates.
