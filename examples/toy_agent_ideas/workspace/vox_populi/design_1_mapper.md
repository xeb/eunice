# Design 1: The Sentiment Mapper (Conservative)

## Purpose
To bridge the gap between user feedback (tweets, reviews, support tickets) and the codebase by generating "Sentiment Heatmaps" that map negative/positive user emotion directly to source code files. This provides developers with a "Sentiment Coverage" metric alongside Code Coverage.

## Loop Structure
1.  **Ingest:** Periodically (cron) run `web_brave_web_search` and `web_brave_news_search` for product keywords + "slow", "bug", "love", "hate", "crash".
2.  **Entity Extraction:** Parse search snippets to extract "Feature Entities" (e.g., "Login", "Export PDF", "Dark Mode").
3.  **Code Mapping:** Use `grep_search` to find which files correspond to these entities (e.g., "Export PDF" -> `src/features/export/PdfGenerator.ts`).
4.  **Graph Update:** Update `memory` graph: `(UserVoice) -[COMPLAINS_ABOUT]-> (FeatureEntity) -[IMPLEMENTED_BY]-> (SourceFile)`.
5.  **Report Generation:** Walk the graph to calculate "Heat" (frequency * sentiment severity) per file. Generate `REPORTS/Sentiment_Heatmap_[Date].md`.

## Tool Usage
*   **web_brave_web_search**: Source of unstructured feedback.
*   **grep_search**: Heuristic mapping of natural language feature names to file paths.
*   **memory_create_entities / relations**: Storing the mapping and cumulative sentiment scores.
*   **filesystem_write_file**: Outputting the static report.

## Memory Architecture
*   **Nodes**: `FeedbackItem` (content, source, sentiment_score), `FeatureConcept` (name), `SourceFile` (path).
*   **Edges**: `MENTIONS`, `MAPS_TO`.
*   **Persistence**: The graph is the source of truth for historical sentiment trends (e.g., "Did Login get better after the v2 release?").

## Failure Modes
*   **Hallucinated Mapping**: Mapping "Button" in a review to a generic `Button.tsx` component instead of the specific context.
    *   *Recovery:* Use "Confidence Scores" in the graph. Only map if specific keywords co-occur (e.g., "Login Button" -> `LoginButton.tsx`).
*   **Noise**: Marketing bots or spam skewing sentiment.
    *   *Recovery:* Filter out sources with repetitive text or exact duplicate content.

## Human Touchpoints
*   **Read-Only**: The agent produces reports. Humans read them to decide on refactoring or prioritization. No autonomous code changes.
