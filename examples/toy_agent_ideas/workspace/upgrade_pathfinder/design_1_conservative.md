# Design 1: The Dependency Auditor (Conservative)

## Purpose
To provide a clear, risk-free visibility layer over a project's technical debt and security vulnerabilities without modifying code. It acts as a specialized linter that quantifies the "interest rate" of the codebase.

## Loop Structure
1. **Trigger:** Scheduled (cron) or Manual (CI pipeline step).
2. **Scan:** Reads manifest files (package.json, pom.xml, requirements.txt) and lock files.
3. **Enrich:** Queries external databases (NVD for CVEs, Registry APIs for latest versions/deprecation notices).
4. **Analyze:** Calculates a "Debt Score" for each dependency based on:
   - Version lag (Major/Minor versions behind).
   - Security severity.
   - "Fan-out" (how many local files import it - using grep).
5. **Report:** Generates a static artifact DEBT_REPORT.md and debt_graph.json.

## Tool Usage
- **filesystem:** Read manifests; search for imports using filesystem_search_files to calculate usage impact.
- **web:** web_brave_web_search and fetch_fetch to retrieve changelogs, release dates, and vulnerability data.
- **shell:** Execute native audit commands (npm audit, pip check) to bootstrap data.
- **memory:** Stores the "Baseline" state to track trends over time (e.g., "Debt increased by 5% this week").

## Memory Architecture
- **Entities:** Library, Version, Vulnerability, File.
- **Relations:** Library -> has_version, File -> imports -> Library, Library -> has_cve -> Vulnerability.
- **Persistence:** Simple JSON dump or Memory Graph.

## Failure Modes
- **API Limits:** Rate limiting on registry APIs. -> Recovery: Cache results for 24h.
- **False Positives:** Flagging internal libraries as unknown. -> Recovery: Allow .debtignore config.

## Human Touchpoints
- **Read-Only:** The agent never writes code. Humans review the generated report and decide when to act.
