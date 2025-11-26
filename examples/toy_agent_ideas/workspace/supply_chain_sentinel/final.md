# Agent: The Supply Chain Sentinel

## Problem Domain
**Holistic Software Supply Chain Health**
Modern applications rely on thousands of transitive dependencies. Current tools (Dependabot, Snyk) focus on **Vulnerabilities (CVEs)**. They miss the **Social Risks** (burnout, single maintainer, bus factor) and **Structural Risks** (dead projects, license drift, bloating).
The "Supply Chain Sentinel" is not just a scanner; it is a **Risk Manager** that models the ecosystem as a living graph.

## Key Insight
**"Transitive Social Risk" + "Local Usage Weight"**
The agent combines two novel signals:
1.  **Social Graph**: It maps `Package -> Maintainer` to calculate a "Bus Factor Risk". If a core library is maintained by one person who hasn't tweeted or committed in 6 months, that is a High Risk, even if there are no CVEs.
2.  **Usage Weight**: It uses `grep` to analyze *how much* of the library you actually use. A heavy dependency on a risky lib is critical; a single unused import is negligible.

## Core Tools
*   **memory**: Stores the "Ecosystem Graph" (Nodes: Package, Person, Repo, License).
*   **web**: Profiles maintainer activity (GitHub, Socials) and searches for alternatives.
*   **grep**: Performs "Usage Analysis" on the local codebase.
*   **filesystem**: Reads manifests and writes "Migration Plans".
*   **fetch**: Retrieves raw metadata from registries.

## Architecture

### 1. The Knowledge Graph (Memory)
The agent builds a persistent graph in the `memory` MCP server:
*   **Entities**: `Package`, `Version`, `Maintainer`, `License`, `Vulnerability`.
*   **Relations**:
    *   `(Package)-[DEPENDS_ON]->(Package)`
    *   `(Package)-[MAINTAINED_BY]->(Maintainer)`
    *   `(App)-[USES_HEAVILY {score: 0.9}]->(Package)`
    *   `(Maintainer)-[STATUS]->(BurnoutRisk)`

### 2. The Execution Loop
The agent runs daily or on PR triggers.

#### Phase A: Cartography (Mapping)
1.  Parses `package.json` / `go.mod` / `Cargo.toml`.
2.  Recursively fetches dependencies to build the full tree.
3.  Updates the Graph. If a node is "stale" (not checked in > 7 days), it schedules a refresh.

#### Phase B: Profiling (The "Private Investigator")
1.  For high-centrality nodes, it performs deep checks:
    *   **Activity**: Last commit? Last release?
    *   **Community**: "Bus Factor" (number of active contributors).
    *   **Sentiment**: Searches web for "Is project X dead?" or "Project X alternatives".
2.  It updates the `RiskScore` property on the Package node.

#### Phase C: Local Context (The "Usage Check")
1.  For "Risky" packages, it scans the local code.
    *   `grep -r "import .* from 'risky-lib'"`
2.  It calculates a `CouplingScore`.
    *   **Low Coupling**: "Safe to remove/replace."
    *   **High Coupling**: "Critical infrastructure risk."

#### Phase D: Governance (The "Gatekeeper")
1.  It calculates the **Total Project Risk Score**.
2.  **Reporting**: Updates `docs/supply_chain_health.md` with a "Weather Report" (e.g., "Storm Approaching: `utils-lib` is unmaintained").
3.  **Action**:
    *   If a Risky Lib has a safe, popular alternative (found via Web), it drafts a **Migration Plan**.
    *   Example: "Suggestion: Replace `moment` with `date-fns`. `moment` is in maintenance mode. You use it in only 3 files."

## Persistence Strategy
**Hybrid**:
*   **Memory Graph**: Stores the evolving understanding of the external ecosystem. This is valuable data that grows over time (e.g., remembering that "Dev X abandoned Project Y").
*   **Filesystem**: Stores snapshots of the report and the "Allowlist/Denylist" configuration.

## Autonomy Level
**High (Monitoring) -> Semi-Autonomous (Proposals)**
*   It autonomously monitors and updates its graph.
*   It **proposes** changes (Migration Plans) but does not execute refactors without approval, as replacing libraries is high-risk.

## Failure Modes & Recovery
1.  **API Rate Limits**: The agent respects `retry-after` headers when querying GitHub/NPM.
2.  **False Alarms**: A "quiet" maintainer isn't always "dead". The agent uses multiple signals (commits + issues + web mentions) to triangulate status.
3.  **Graph Bloat**: The agent prunes "leaf nodes" (transitive deps with low risk) from memory to keep the graph performant, focusing detail on the "Critical Path".
