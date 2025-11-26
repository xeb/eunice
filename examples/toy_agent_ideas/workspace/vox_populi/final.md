# Agent: The Vox Populi

## Purpose
"The Vox Populi" acts as a bidirectional bridge between the **User's Experience** (mined from the web/support) and the **Developer's Codebase**. It solves the problem of "Emotional Latency"—where developers only realize a feature is hated months after deployment. It quantifies **"Sentiment Coverage"** (which code modules generate the most pain) and actively prioritizes technical debt based on real user frustration.

## Core Toolset
*   **web** (`brave_web_search`, `brave_news_search`): To monitor social media, forums, and app stores for feedback.
*   **grep** (`grep_search`): To map natural language feature descriptions to specific source files.
*   **memory** (`memory_create_entities`, `memory_search_nodes`): To maintain the persistent "Sentiment Graph".
*   **filesystem** (`write_file`): To generate reports and draft issues.

## Architecture

### 1. The Sentiment Graph (Persistence)
The agent maintains a knowledge graph that links three domains:
*   **Social Domain:** `(UserVoice) -[POSTED]-> (Complaint) -[HAS_SENTIMENT]-> (Score)`
*   **Feature Domain:** `(Complaint) -[ABOUT]-> (FeatureConcept)`
*   **Code Domain:** `(FeatureConcept) -[IMPLEMENTED_BY]-> (SourceFile)`

*Key Insight:* By calculating the centrality and weight of the `SourceFile` nodes based on `Complaint` volume, we can generate a **"Heatmap of Hate"** for the codebase.

### 2. The Execution Loop (Autonomous)
1.  **Listen:** Every 6 hours, search for product keywords + sentiment markers ("broken", "slow", "love", "wish").
2.  **Map:** Use LLM + Grep to trace the feedback to a file.
    *   *Input:* "The PDF export is so slow!"
    *   *Action:* `grep "export" && grep "pdf"` -> `src/features/pdf/ExportJob.ts`
    *   *Graph:* Add edge `(Complaint) -> (ExportJob.ts)`
3.  **Quantify:** Update the `SentimentScore` of `ExportJob.ts`.
4.  **Act:**
    *   **Threshold Trigger:** If `ExportJob.ts` drops below -0.7 sentiment, check if an Issue exists.
    *   **Drafting:** If no issue exists, create `issues/drafts/REGRESSION_PDF_EXPORT.md` with a summary of the 50 tweets.
    *   **Enrichment:** If an issue exists, post a comment: "Severity Check: 10 new complaints today. Is this priority correct?"

### 3. The Empathy Layer (On-Demand)
When a developer opens a PR touching `ExportJob.ts`, the CI runs `vox_populi --audit`.
The agent checks the Sentiment Graph and comments:
> "⚠️ **High User Frustration Detected**
> This file is associated with 'PDF Slowness' (Sentiment: -0.8).
> *Top User Quote:* 'I lose my work every time I try to export.'
> **Recommendation:** Ensure no performance regressions."

## Failure Modes & Recovery
1.  **Misattribution:** Users complain about "Login" but the bug is in the "API Gateway".
    *   *Mitigation:* The agent marks mappings as "Probabilistic". If code changes don't fix the sentiment, the agent widens the search radius in the graph.
2.  **Sentiment Spikes:** A viral marketing campaign might look like a feature spike.
    *   *Mitigation:* The agent ignores "Broad" sentiment (brand level) and focuses on "Specific" sentiment (feature level) by filtering for functional verbs.

## Human Touchpoints
*   **Draft Review:** Humans approve the "Draft Issues" the agent creates.
*   **Graph Gardening:** Developers can manually correct `(Feature) -> (File)` links to improve future accuracy.
