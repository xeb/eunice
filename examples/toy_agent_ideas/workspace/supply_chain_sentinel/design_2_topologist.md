# Design 2: The Risk Topologist (Innovative)

## Purpose
A graph-native agent that models the **Social and Structural** topology of the software supply chain. It goes beyond metadata to understand *relationships* and *bus factors*. It answers: "If Maintainer X gets hit by a bus, what % of my stack collapses?"

## Loop Structure
1.  **Ingestion & Graphing**:
    *   Parses dependencies and creates Nodes in the **Memory Graph**: `(Package)`, `(Maintainer)`, `(Repository)`.
    *   Creates Relations: `(Package)-[MAINTAINED_BY]->(Maintainer)`, `(Package)-[HOSTED_AT]->(Repository)`.
2.  **Social Reconnaissance**:
    *   Selects high-centrality Maintainer nodes.
    *   Uses `web_brave_web_search` to profile their activity (GitHub, Twitter/X, Blogs).
    *   Updates Node properties: `burnout_risk`, `recent_activity_score`.
3.  **Local Coupling Analysis**:
    *   Uses `grep` to find every import/usage of the package in the *local* codebase.
    *   Calculates a **Dependency Weight**: How entangled is this library?
    *   Updates Edge: `(App)-[DEPENDS_HEAVILY_ON {usage_count: 500}]->(Package)`.
4.  **Risk Propagation**:
    *   Runs a graph algorithm (like PageRank) to calculate a **Composite Risk Score**.
    *   Risk = (Maintainer Risk * Transitive Depth) + Local Coupling.

## Tool Usage
*   **memory**: Stores the complex graph of Packages, People, and Usage.
*   **grep**: Analyzes local code to quantify dependency usage.
*   **web**: Deep searches on maintainer health and project viability.
*   **filesystem**: Reads code.

## Memory Architecture
*   **Persistent Graph**: The `memory` MCP server holds the "World Model" of the supply chain.
*   **Evolution**: Tracks risk scores over time. "Package X is becoming riskier (Score 0.4 -> 0.8)".

## Failure Modes
*   **Privacy Limits**: Cannot determine maintainer health if profiles are private.
*   **Misinterpretation**: Might flag a "stable" project as "abandoned".
*   **Graph Explosion**: Large dependency trees (like `node_modules`) might overwhelm the memory graph if not pruned.

## Human Touchpoints
*   **Dashboard**: The agent maintains a `dashboard.md` updated in real-time.
*   **Notifications**: Creates an "Alert" file if a critical path dependency becomes high-risk.
