# Design 1: The Passive Observer (Conservative)

## Purpose
A specialized "Librarian of Norms" that analyzes a digital community's public archives (forums, issue trackers, mailing lists) to produce a **"Cultural Dossier"** for new entrants. It answers: "Who matters here?", "What topics are taboo?", and "What is the preferred communication style?"

## Loop Structure
1.  **Ingest:** User provides a target URL (e.g., a GitHub repository or Discourse forum).
2.  **Scrape & Parse:** Agent uses `curl`/`brave_search` to fetch recent threads, comments, and guidelines.
3.  **Entity Extraction:** Identifies recurring actors (Users), topics (Keywords), and sentiment patterns.
4.  **Norm Inference:**
    *   *Observation:* User A posted "X" and got downvoted/closed. -> *Inference:* "X is discouraged."
    *   *Observation:* User B posted "Y" and got merged/praised. -> *Inference:* "Y is encouraged."
5.  **Graph Synthesis:** Updates the Memory Graph with nodes for `Norm`, `Jargon`, `Influencer`.
6.  **Report Generation:** Writes a `README_CULTURE.md` file summarizing the findings.

## Tool Usage
*   **Web:** Brave Search to find community archives and "rules" pages.
*   **Shell:** `curl` to fetch raw text; `grep` to find patterns like "closing this because".
*   **Memory:** Stores specific rules linked to evidence URLs.
    *   Entities: `Community`, `Norm`, `User`, `Term`.
    *   Relations: `DISCOURAGES`, `REWARDS`, `USES_JARGON`.

## Memory Architecture
*   **Graph-Based Truth:** The "Culture" is a graph.
    *   (Community: Rust) -> (Norm: No Unsafe Code) -> (Evidence: URL to Issue #123)
*   **Decay:** Old norms (from 3 years ago) have lower weight than recent ones.

## Failure Modes
*   **Misinterpretation:** Sarcasm or inside jokes might be interpreted as literal rules. *Recovery:* User can manually flag a "Norm" node as incorrect in the graph.
*   **Inaccessibility:** Private Discords/Slacks cannot be scraped. *Recovery:* Fails gracefully with "Public archives only" warning.

## Human Touchpoints
*   **Target Selection:** Human defines the scope.
*   **Report Review:** Human reads the generated dossier.
