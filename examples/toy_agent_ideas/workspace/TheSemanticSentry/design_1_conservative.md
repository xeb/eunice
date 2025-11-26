# Design 1: The Log Curator (Conservative)

## Purpose
A semi-autonomous assistant that helps sysadmins reduce "alert fatigue" by categorizing log messages based on external knowledge. It strictly requires human verification before suppressing any alerts.

## Loop Structure
1. **Ingest:** Periodically reads specific log files (e.g., `/var/log/syslog`).
2. **Filter:** Checks each line against a `memory` graph of "Known Patterns".
3. **Research:** For unknown patterns, extracts keywords and performs a `web_search`.
4. **Propose:** Generates a daily "Triage Report" grouping new errors with found web explanations (e.g., "StackOverflow thread #123 says this is harmless").
5. **Learn:** User reviews the report and tags patterns as "Ignore", "Watch", or "Critical".
6. **Persist:** Updates the `memory` graph with the user's decision.

## Tool Usage
- **grep:** Used to scan files for specific error levels (WARN, ERROR).
- **web_search:** Searches for error strings to find context.
- **memory:** Stores entities `LogPattern` with properties `status` (Benign/Critical) and `evidence` (URLs).
- **filesystem:** Reads log files.

## Memory Architecture
- **Nodes:** `LogPattern`, `SystemComponent`, `WebSource`.
- **Relations:** `(LogPattern) -> HAS_STATUS -> (Status)`, `(LogPattern) -> EXPLAINED_BY -> (WebSource)`.

## Failure Modes
- **Hallucination:** Might misinterpret a search result (e.g., confusing a "fixed in v2" bug with a "critical security hole").
- **Recovery:** Human verification step prevents this from affecting the system.

## Human Touchpoints
- **Daily Review:** User must manually approve "Ignore" rules.
- **Config:** User defines which log files to watch.
