# Design 1: The Incubation Assistant (Conservative)

## Purpose
To reduce the cognitive load of "background research" by autonomously gathering relevant materials for a defined problem while the user is away, simulating a "fresh perspective" upon return.

## Loop Structure
1.  **Input Phase**: User writes a problem description into `inbox/problem_statement.md`.
2.  **Dormant Phase**: Agent waits for a scheduled time (e.g., 2 AM) or a signal (system idle).
3.  **Research Phase**:
    *   Parses `problem_statement.md`.
    *   Extracts keywords.
    *   Performs `web_brave_web_search` for standard definitions, tutorials, and discussions.
    *   Performs `web_brave_news_search` for recent developments.
4.  **Synthesis Phase**:
    *   Summarizes findings into `research/summary_<date>.md`.
    *   Groups links by category (Academic, Practical, News).
5.  **Wake Phase**: Sends a notification (or updates a status file) indicating research is complete.

## Tool Usage
*   **filesystem**: Reading the problem, writing the summary.
*   **web**: Searching for content.
*   **memory**: Simple key-value storage to track which URLs have already been visited (to avoid duplicates).

## Memory Architecture
*   **Graph**: Minimal.
    *   Entities: `Problem`, `Source`.
    *   Relations: `Problem -> has_source -> Source`.
*   **Persistence**: Mostly file-based output. Memory is just for deduplication.

## Failure Modes
*   **Irrelevance**: Search results are off-topic. (Recovery: User updates problem keywords).
*   **Empty Results**: Niche topic. (Recovery: Agent broadens search terms automatically).

## Human Touchpoints
*   **Initiation**: User must explicitly define the problem.
*   **Review**: User reads the markdown summary.
