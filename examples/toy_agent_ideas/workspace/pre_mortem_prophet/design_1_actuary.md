# Design 1: The Actuary (Conservative/Analytical)

## Purpose
This agent automates the risk assessment phase of software architecture. Instead of relying on manual threat modeling or "what if" brainstorming, it systematically scans the codebase, identifies dependencies, and queries external vulnerability databases (CVEs) and outage reports to calculate a "Fragility Score" for each component.

## Loop Structure
1.  **Inventory**: Scan package.json, Cargo.toml, docker-compose.yml, etc., to build a Bill of Materials (BOM).
2.  **External Correlation**: Search the web for "outage [library name]", "known issues [database version]".
3.  **Risk Calculation**: Score components based on:
    *   Time since last update.
    *   Frequency of appearance in external outage reports.
    *   Centrality in the internal dependency graph (using grep to see import counts).
4.  **Reporting**: Generate a risk_report.md sorted by probability of failure.

## Tool Usage
*   **filesystem**: Read config files and source code to map the system.
*   **web**: Search for "Redis 6.2 failure modes", "AWS us-east-1 outage history".
*   **memory**: Store the "Risk Graph" (Node: Component, Edge: Dependency, Attribute: Risk Score).

## Memory Architecture
*   **Entities**: Component, ExternalDependency, RiskFactor.
*   **Relations**: DEPENDS_ON, VULNERABLE_TO.
*   **Retrieval**: Query for "High Risk Components" to prioritize refactoring.

## Failure Modes
*   **False Positives**: Flagging stable, mature libraries as "risky" just because they are old.
*   **Noise**: Overwhelming the user with low-probability CVEs.
*   **Recovery**: If web search fails, rely on internal static analysis heuristics.

## Human Touchpoints
*   **Read-Only**: The agent produces a report. Humans decide if the risk is acceptable.
*   **Configuration**: Humans can whitelist/ignore specific risks via a config file.
