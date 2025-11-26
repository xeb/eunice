# Design 3: The Meme War (Evolutionary)

## Purpose
To treat code patterns as "Memes" competing for survival. Instead of top-down standardization, it visualizes "Winning" vs "Dying" patterns, encouraging organic convergence.

## Loop Structure
1.  **Census**: Periodically scans all projects for specific patterns (e.g., library imports, error handling blocks, linter rules).
2.  **Leaderboard Calculation**:
    *   `axios` used in 8/10 projects.
    *   `fetch` used in 2/10 projects.
3.  **Pressure Application**:
    *   **Winners**: If a pattern is >80% dominant, the agent opens "Deprecation Issues" on the 20% minority projects.
    *   **Losers**: If a pattern is <20% and shrinking, it suggests removal.
4.  **Dashboarding**: Maintains a `STATUS.md` file in the root `workspace` showing the "Evolutionary Health" of the ecosystem.

## Tool Usage
*   `grep_count-matches`: To gather stats efficiently.
*   `memory`: To track historical trends (is `fetch` usage growing or shrinking?).
*   `filesystem`: To write the Dashboard.

## Memory Architecture
*   **Nodes**: `Pattern`, `AdoptionStat` (time-series).
*   **Edges**: `IS_GROWING`, `IS_DYING`, `SUPERCEDES`.
*   **Insight**: Tracks the *trajectory* of technology choices, preventing "Resume Driven Development" by showing what the *team* actually uses.

## Failure Modes
*   **Tyranny of the Majority**: The agent might suppress a valid new technology just because it's rare (start of the S-curve).
*   **Mitigation**: "Incubator" whitelist where new patterns are protected from deprecation for X months.

## Human Touchpoints
*   **Observation**: Humans read the dashboard.
*   **Compliance**: Humans decide whether to obey the "Deprecation Warnings".
