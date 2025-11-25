# Design 1: The Sentinel (Conservative)

## Purpose
The Sentinel acts as a highly intelligent, non-intrusive security camera for the codebase. Its primary goal is to audit dependencies, assess risk based on global data (web) and local history (memory), and alert the team to toxic packages without modifying code.

## Loop Structure
1. **Patrol:** Run on a schedule (e.g., daily).
2. **Scan:** Parse `package.json`, `requirements.txt`, etc., to build a dependency tree.
3. **Intelligence Gathering:**
   - Query `memory` to see if we've checked these versions recently.
   - If new/stale, use `web` to search for changelogs, CVEs, and GitHub issue discussions regarding recent versions.
4. **Risk Assessment:**
   - Calculate a "Health Score" for each dependency.
   - Factors: Age, release frequency, open issues, CVEs, reputation in the memory graph.
5. **Report:**
   - Generate a Markdown report in `reports/` highlighting critical vulnerabilities or "safe" updates.
   - Update `memory` with findings (e.g., "Lib X v1.2.0 has high risk of memory leaks").

## Tool Usage
- **filesystem:** Read-only access to manifest files.
- **web:** Brave search for "Lib X v1.2.0 breaking changes", "Lib X vulnerability".
- **memory:** Stores entities (Package, Version) and relations (HAS_VULNERABILITY, IS_COMPATIBLE_WITH).
- **shell:** Minimal usage, perhaps to run `npm audit` or `pip check` for raw data.

## Memory Architecture
- **Nodes:** `Package`, `Version`, `Vulnerability`, `Report`.
- **Edges:** `DEPENDS_ON`, `HAS_ISSUE`, `RECOMMENDS_UPDATE`.
- **Persistence:** The graph grows over time, becoming a local database of "trusted" vs "suspicious" software supply chain elements.

## Failure Modes
- **Hallucination:** Might flag a safe package as dangerous based on vague search results.
- **Mitigation:** Links to sources are mandatory in reports. Humans verify.
- **Staleness:** If the agent crashes, it resumes from the last checked timestamp in memory.

## Human Touchpoints
- **Read-Only:** The agent never commits code.
- **Notification:** Alerts posted to a dashboard or file.
- **Approval:** None required for operation, only for acting on the report.
