# Design 2: The Strategic Portfolio Manager

## Purpose
The **Strategic Portfolio Manager** goes beyond simple listing of dependencies. It aims to understand the *purpose* of each library and align local usage with global trends. It answers the question: "Are we solving modern problems with archaic tools?" by building a semantic map of "Problem Domain" vs. "Tool Solution".

## Loop Structure
1.  **Semantic Mapping:**
    - The agent uses `grep` to find how libraries are imported and used in the code (e.g., `import moment from 'moment'`).
    - It uses `web_search` to identify the *category* of the library (e.g., "Moment.js is a Date/Time manipulation library").
    - It stores this in the **Memory Graph**: `(Entity: Moment.js) --[IS_A]--> (Category: Date Library)`.
2.  **Trend Analysis:**
    - For each category identified, the agent searches for "Best Date Library JavaScript 2025" or checks GitHub stars/trends.
    - It identifies the current "industry standard" (e.g., `date-fns`, `Luxon`, or native `Temporal` API).
    - It compares the project's choice against the industry standard.
3.  **Gap Analysis & Recommendation:**
    - If a discrepancy is found (e.g., Project uses `request`, Industry uses `axios` or `fetch`), the agent calculates a "Migration Score".
    - It creates a proposal: "Recommendation: Migrate from `request` to `axios`. Benefit: Security, Promise-based API. Effort: Medium (45 occurrences)."

## Tool Usage
-   **memory:** Stores the ontology of libraries, their categories, and their "Health Status" (Adopt, Trial, Assess, Hold).
-   **grep:** Analyzes code density—how heavily coupled is the codebase to a specific library?
-   **web_brave_web_search:** Gathers qualitative "sentiment" and "trend" data.

## Memory Architecture
-   **Graph-Based Persistence:**
    - Nodes: `Library`, `Category`, `File`, `Vulnerability`.
    - Edges: `DEPENDS_ON`, `REPLACES`, `USED_IN`.
    - Allows queries like: "Show me all 'HTTP Client' libraries we use across all repos."

## Failure Modes
-   **Misclassification:** It might mistake a custom internal library for a public one or miscategorize a utility. *Mitigation:* Human review step for the initial graph population.
-   **Hallucinated Trends:** It might follow a hype cycle too closely. *Mitigation:* Require multiple sources (GitHub stars + NPM downloads + Blog mentions) to confirm a trend.

## Human Touchpoints
-   **Strategic Review:** The agent presents a "Quarterly Strategy Report" for the Tech Lead to approve.
-   **Feedback:** Humans can mark a library as "Legacy - Do Not Migrate" in the graph to stop nagging.

## Pros & Cons
-   **Pros:** Provides actionable architectural advice, not just data. Helps pay down technical debt proactively.
-   **Cons:** Complex to implement (requires accurate categorization); high token cost for web searching and reasoning.
