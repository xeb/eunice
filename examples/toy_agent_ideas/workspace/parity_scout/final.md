# Agent: The Parity Scout

## Purpose
The Parity Scout is an autonomous "Product Intelligence" agent that prevents SaaS products from falling behind the competition. It continuously monitors competitor public documentation, extracts "Features" into a structured ontology, and cross-references them against the local codebase to generate a real-time "Feature Gap Analysis."

It answers the question: **"What did Stripe ship this week that we don't have yet?"**

## Core Loop
1.  **Surveillance (Web/Fetch)**:
    *   The agent monitors a user-defined list of competitor URLs (Pricing, Changelog, API Docs).
    *   It detects meaningful semantic additions (e.g., "Added support for Webhooks") rather than just HTML diffs.

2.  **Ontology Construction (Memory)**:
    *   It updates a persistent **Memory Graph** with nodes for `Competitor`, `Feature`, and `PricingTier`.
    *   Example: `(Stripe) -> [HAS_FEATURE] -> (Webhooks)`.

3.  **Grounding (Grep/Filesystem)**:
    *   For each feature in the graph, the agent attempts to "ground" it in the local codebase.
    *   It generates semantic search patterns (e.g., for "Webhooks", it greps for `webhook`, `callback`, `event_handler`).
    *   If matches are found, it creates a relationship: `(LocalProject) -> [IMPLEMENTS] -> (Webhooks)`.

4.  **Gap & Opportunity Analysis**:
    *   **The Gap**: Competitor has it, we don't. (Action: Create Issue).
    *   **The Hidden Gem**: We have it (found in code), but our public docs don't mention it. (Action: Flag for Marketing).
    *   **The Zombie**: We have code for it, but it's dead/commented out.

5.  **Reporting**:
    *   Maintains a live `docs/COMPETITIVE_MATRIX.md` file in the repo.
    *   Optionally drafts GitHub Issues for critical missing features.

## Tool Usage Details
*   **web**: Used to search for "Competitor X API documentation" if URLs aren't provided.
*   **fetch**: Downloads the raw competitor pages.
*   **memory**: Stores the "World Model" of features. This is crucial because "Webhooks" is the same concept across all competitors, so the graph naturally dedupes them.
*   **grep**: The primary sensor for "Self-Awareness". It allows the agent to know what the software *actually does* vs what the docs say.
*   **filesystem**: Writes the final Markdown reports and reads local source code.

## Persistence Strategy
*   **Hybrid**:
    *   **Memory Graph**: Stores the evolving ontology of features and competitor states.
    *   **Filesystem**: Stores the "Evidence" (HTML snapshots) and the "Output" (Markdown reports).

## Autonomy Level
**High (Background Daemon)**. The agent runs silently. It does not modify code (safe). It only adds/modifies documentation files (`docs/`) and potentially drafts issues. It requires no human intervention to run, but human judgment to act on the findings.

## Key Insight
**Comparative Feature Extraction**. Most competitive analysis tools are external dashboards. The Parity Scout runs *inside* your repo, using the codebase itself as the ground truth for "What we have," enabling a level of precision (Code vs. Marketing Copy) that external tools cannot match.
