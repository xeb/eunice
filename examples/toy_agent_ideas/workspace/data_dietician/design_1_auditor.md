# Design 1: The Data Auditor (Conservative)

## Purpose
To map the "nutritional value" of the user's filesystem without modifying it. It identifies digital bloat, duplicate data, and "dead" sectors of the hard drive, providing the user with a dashboard of their digital health.

## Core Loop
1. **Crawl:** Recursively lists directories.
2. **Analyze:** checks `last_accessed` and `size` metadata.
3. **Remember:** Creates entities in the Memory Graph for each file, tagging them with `Frequency`, `Type`, and `Age`.
4. **Report:** Generates a Markdown report `diet_report.md` highlighting:
   - "Heavy" directories (High size, low access)
   - "Stale" clusters (Groups of files untouched for >1 year)
   - "Duplicate" candidates (based on size/name)

## Tool Usage
- `filesystem`: Read-only access to list files and get metadata.
- `memory`: Stores the file index. This allows the agent to track *changes* over time (e.g., "This folder is growing by 500MB/week").
- `shell`: Uses `du`, `find`, or `md5sum` for efficient low-level analysis.

## Memory Architecture
- **Nodes:** `File`, `Directory`, `Extension`.
- **Edges:** `CONTAINED_IN`, `HAS_DUPLICATE`, `ACCESSED_AT`.
- **Observations:** "File X has not been opened since 2022."

## Failure Modes
- **Privacy Leak:** Metadata sent to memory might contain sensitive filenames. *Mitigation:* Allow user to blacklist directories.
- **Performance:** Scanning millions of files is slow. *Mitigation:* Incremental scanning (only check modified timestamp).

## Human Touchpoints
- **Reporting:** The user only interacts with the generated Markdown report.
- **Action:** The user must manually delete or move files based on the recommendations.
