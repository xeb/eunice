# Design 3: The Multi-Perspective Historian (Hybrid)

## Purpose
The **Multi-Perspective Historian** acknowledges that "objective truth" is rare in complex narratives. Instead of trying to find *the* story, it captures *multiple* perspectives. It uses a "Newsroom" of sub-agents (Persona-based) to debate the significance of news, producing a synthesized report that highlights consensus and contention.

## Loop Structure
1.  **News Gathering:** Fetches news on a topic from diverse sources (Left, Right, Center, International).
2.  **The Editorial Board:**
    *   Spawns virtual personas (e.g., "The Skeptic", "The Technologist", "The Economist").
    *   Each persona analyzes the news through their lens and writes a "Take".
3.  **Synthesis & Debate:**
    *   The "Editor" agent reviews the Takes.
    *   Identifies "Points of Agreement" and "Points of Contention".
    *   Weaves a "Balanced Narrative" that explicitly cites which perspective holds which view.
4.  **Archival:**
    *   Stores the "Debate Transcript" and the "Final Report" in the filesystem.
    *   Updates the Memory Graph with the "Credibility Score" of sources based on how often their claims held up over time.

## Tool Usage
*   **web:** `web_brave_news_search` (with site-specific queries to ensure diversity).
*   **memory:** Used for "Source Reputation" and "Persona Context".
*   **filesystem:** Stores the rich text outputs (Debates, Reports).
*   **shell:** Could be used to run local LLMs if available for cheaper "Persona" generation, but assumes API for now.

## Memory Architecture
*   **Hybrid:** 
    *   **Graph:** Tracks Sources and their reliability/bias (Meta-knowledge).
    *   **Filesystem:** Stores the actual narrative content (The Stories).
*   **Source-Centric:** The memory focuses on *who* said what, rather than just *what* happened.

## Failure Modes
*   **Echo Chamber:** If diverse sources aren't found, the personas have nothing to debate.
    *   *Mitigation:* Active search for "opposing views" if the retrieved set is too homogeneous.
*   **Cacophony:** The debate becomes too messy to synthesize.
    *   *Mitigation:* Strict "Moderator" prompt to force structured output from personas.

## Human Touchpoints
*   **Persona Configuration:** User defines who sits on the "Editorial Board".
*   **Adjudication:** User can settle debates if they want to force a specific narrative direction.
