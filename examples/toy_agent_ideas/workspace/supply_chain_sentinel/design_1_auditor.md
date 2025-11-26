# Design 1: The Dependency Auditor (Conservative)

## Purpose
A lightweight, stateless tool that acts as an advanced "linter" for supply chain health. It runs on a schedule (e.g., weekly) or via CI trigger, generating a human-readable report. It focuses on **visibility** rather than autonomy.

## Loop Structure
1.  **Discovery**:
    *   Reads manifest files (`package.json`, `requirements.txt`, `Cargo.toml`).
    *   Constructs a flat list of direct and transitive dependencies (using lockfiles).
2.  **Enrichment**:
    *   For each dependency, performs a `fetch` or `web_brave_web_search` to gather:
        *   Last commit date.
        *   Open issue count.
        *   Number of maintainers.
        *   Known CVEs (using public DBs).
3.  **Analysis**:
    *   Applies static thresholds (e.g., "Last commit > 1 year ago = HIGH RISK").
    *   Checks for "License Drift" (e.g., MIT changed to AGPL).
4.  **Reporting**:
    *   Generates `reports/audit_YYYY-MM-DD.md`.
    *   Highlights "Top 5 Riskiest Packages".

## Tool Usage
*   **filesystem**: Read manifests, write reports.
*   **web**: Query package registries (NPM, PyPI, Crates.io) and CVE databases.
*   **shell**: Execute package manager commands (e.g., `npm list`) to get the dependency tree.

## Memory Architecture
*   **Stateless**: Does not maintain a graph.
*   **Cache**: Uses a temporary file cache (`.cache/auditor.json`) to avoid re-fetching metadata for unchanged packages during a single run.

## Failure Modes
*   **Registry Downtime**: Fails gracefully, reporting "Unknown" status.
*   **Rate Limiting**: Pauses execution if web requests are blocked.
*   **False Positives**: Flags "finished" libraries (stable, no updates needed) as "dead".

## Human Touchpoints
*   **Read-Only**: The agent never modifies code.
*   **Alerting**: The user reads the generated Markdown report.
