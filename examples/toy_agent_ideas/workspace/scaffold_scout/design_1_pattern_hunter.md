# Design 1: The Pattern Hunter (Conservative)

## Purpose
To passively analyze the codebase for structural repetition and report opportunities for standardization, without modifying code or creating templates automatically.

## Loop Structure
1. **Scheduled Scan:** Runs once daily or on `git push`.
2. **Fingerprinting:** 
   - Walks the file system.
   - Generates a "structural hash" for each file (stripping comments, strings, and whitespace).
   - Groups files with high similarity (>80%).
3. **Directory Analysis:**
   - Checks if groups of files appear in similar directory structures (e.g., `feature/api.ts` and `feature/model.ts`).
4. **Reporting:**
   - Generates a Markdown report: `workspace/scaffold_report.md`.
   - Lists "Candidate Templates" (e.g., "Detected 15 instances of 'Redux Slice' pattern").

## Tool Usage
- **filesystem (via shell):** `find` and `cat` to ingest code.
- **shell:** `diff` or custom scripts to compare file structures.
- **memory:** Stores the hash map of the previous run to highlight *new* patterns.

## Memory Architecture
- **Nodes:** `FileStructure` (abstract syntax shape).
- **Edges:** `HAS_INSTANCE` pointing to actual file paths.
- **Persistence:** Simple graph serialization.

## Failure Modes
- **False Positives:** Identifies two empty files as a "pattern".
- **Noise:** Overwhelms the user with obvious duplicates (e.g., migration files).
- **Mitigation:** Uses a "Complexity Threshold" (must be >10 lines) to report.

## Human Touchpoints
- **Read-Only:** The agent never creates files.
- **Review:** User reads the report and decides if they want to create a template manually.
