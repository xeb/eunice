# Design 2: The Sanction Sentinel (Innovative/Active)

## Purpose
An active gatekeeper that prevents "risky" code from entering the codebase at the commit/install time. It treats geopolitical risk as a compile-time error. It uses a persistent graph to track the reputation and allegiance of thousands of maintainers across the open-source ecosystem.

## Loop Structure
1.  **Monitor**: Hooks into the file system (via `fswatch` or pre-commit hook) or CI pipeline.
2.  **Intercept**: When `package.json` changes or `npm install` runs:
    *   Calculates the "Sovereignty Score" of new dependencies.
3.  **Graph Lookup**:
    *   Checks the `memory` graph for existing entities (Maintainers, Companies).
    *   If unknown, triggers a high-priority `web` research task.
4.  **Inference**:
    *   Uses graph pathfinding: `Package -> Maintainer -> Company -> Country -> SanctionList`.
    *   Detects "Hostile Takeovers": If a package changes ownership from a high-trust node to a low-trust/unknown node.
5.  **Act**:
    *   **Block**: Returns exit code 1 to stop the build.
    *   **Quarantine**: Moves the risky package to a sandbox for manual inspection.

## Tool Usage
*   **memory**: Stores a massive graph of `(Person) -[MAINTAINS]-> (Package)`, `(Person) -[LIVES_IN]-> (Country)`, `(Country) -[HAS_STATUS]-> (Sanctioned)`.
*   **shell**: Executes `npm audit` but wraps it with geopolitical checks; Blocks commits.
*   **web**: Continuous background scraping of tech news for "supply chain attack" or "package acquisition" stories.

## Memory Architecture
*   **Graph Database**: Essential. The agent builds a global view of the ecosystem. It knows that "UserA" on GitHub is "UserB" on Twitter and works for "CorpZ".
*   **Persistence**: The graph grows over time, becoming a valuable asset for the organization.

## Failure Modes
*   **Deadlock**: Blocking a critical update due to a false alarm.
*   **Mitigation**: "Break Glass" command allows admins to bypass checks for specific transactions, recording the override in the memory graph as an "Accepted Risk".

## Human Touchpoints
*   **Override**: Interactive CLI prompt: "Warning: Package 'left-pad' is now owned by 'Unknown Entity'. Proceed? [y/N]".
*   **Training**: Humans can add "Trusted" labels to specific nodes in the graph.
