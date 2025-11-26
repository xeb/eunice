# Design 3: The Gardener (Hybrid)

## Purpose
An observation-driven agent that "lives" in the background, watching for "weeds" (growing complexity) as they happen. It doesn't do massive refactors, but nudges the codebase towards health with small, incremental PRs based on trends, not just static snapshots.

## Core Loop
1. **Observation**: 
   - Uses shell to check git diff stats and git log.
   - Uses grep to count complexity metrics (indentation levels, file length).
2. **Trending**: Updates a "Health Record" in Memory for each active file.
   - "File Auth.ts grew by 200 lines this week."
   - "Function processData has increased indentation depth."
3. **Thresholding**: When a trend crosses a threshold (e.g., "Rapid Growth"), it triggers a specific Micro-Refactor.
4. **Micro-Refactor**:
   - Extract Method: Propose moving a large block to a helper function.
   - Add Test: If coverage drops, propose a skeleton test file.
   - Add Docstring: If a complex function is added without docs, generate them.
5. **Engagement**: 
   - Instead of just opening a PR, it might first leave a comment on an existing PR or create a draft PR titled "Suggestion: Split Auth.ts".

## Tool Usage
*   **memory**: Stores *trends* and *history* (e.g., "File Growth Rate"), not the whole AST.
*   **filesystem**: Monitors file sizes and modification times.
*   **text-editor**: Makes small, surgical edits.
*   **shell**: Git history analysis.

## Memory Architecture
*   **Time-Series Proxy**: Entities represent Files. Observations record "Complexity Snapshots" with timestamps.
*   **Goal**: Detect *velocity* of debt, not just presence.

## Failure Modes
*   **Nagging**: Can become annoying if it flags every small change. Needs "cool-down" periods per file.
*   **Context Missing**: Might suggest splitting a cohesive logic block.

## Human Touchpoints
*   **Opt-In**: Developers can tag the Gardener in PRs (@debt-gardener analyze).
*   **Feedback**: "Dismiss" interactions train the agent to ignore certain files.
