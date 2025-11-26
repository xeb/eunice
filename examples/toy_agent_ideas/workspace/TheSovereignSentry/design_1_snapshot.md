# Design 1: The Snapshot Analyst (Conservative)

## Purpose
This agent performs a periodic "Geopolitical Audit" of a software project's supply chain. Instead of interfering with the build process, it generates a static risk report for compliance officers and engineering managers. It focuses on observability and legal compliance (e.g., export controls, sanctions).

## Loop Structure
1.  **Trigger**: Scheduled (cron) or manual invocation via CLI.
2.  **Scan**: Parses manifest files (`package.json`, `go.mod`, etc.) to build a dependency tree.
3.  **Enrichment**:
    *   Queries NPM/PyPI/GitHub APIs (via `fetch`) to get maintainer metadata (email, bio, timezones).
    *   Uses `web_brave_web_search` to correlate emails/usernames with real-world identities and locations.
    *   Uses `web_brave_web_search` to check for corporate acquisitions of packages (e.g., "Who owns library X?").
4.  **Analysis**: compares findings against a configurable `policy.yaml` (e.g., "Flag entries from Country X", "Flag maintainers with < 2 years history").
5.  **Reporting**: Writes a `SUPPLY_CHAIN_RISK.md` file in the repo root and logs a summary to the console.

## Tool Usage
*   **filesystem**: Read manifest files; Write report.
*   **fetch**: Call registry APIs (NPM, PyPI) to get raw metadata.
*   **web**: Search for "Maintainer Name location", "Library X acquisition", "Company Y sanctions".
*   **grep**: Not heavily used, mainly for local keyword scanning in `node_modules` (e.g., looking for Cyrillic/Chinese characters in comments if that's a risk heuristic).

## Memory Architecture
*   **Stateless**: This design is primarily stateless to ensure reproducibility. It re-verifies data on every run or uses a simple local JSON cache file to avoid hitting API rate limits.

## Failure Modes
*   **False Positives**: Flagging a maintainer as "Risky" because they moved or use a VPN.
*   **API Rate Limits**: GitHub/NPM API throttling.
*   **Resolution**: The agent logs "Unknown" for entities it can't verify rather than blocking.

## Human Touchpoints
*   **Review**: Humans read the `SUPPLY_CHAIN_RISK.md` report.
*   **Config**: Humans update `policy.yaml` to whitelist trusted maintainers.
