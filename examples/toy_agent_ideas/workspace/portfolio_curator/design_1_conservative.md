# Design 1: The Git-Biographer (Conservative)

## Purpose
A locally-running agent that generates a "Living Changelog" of your career. It parses your git commit messages across all local repositories and turns them into a human-readable "Work Log" or "Brag Document" suitable for performance reviews.

## Loop Structure
1. **Scan:** Iterates through a user-defined list of directories (`~/projects`).
2. **Parse:** Runs `git log` with specific filters (author=me, date=last_week) to extract commit messages and diff stats.
3. **Categorize:** Uses simple keyword matching (Regex) to classify commits (e.g., "fix" -> Bug Fixes, "feat" -> New Features, "refactor" -> Maintenance).
4. **Report:** Appends a summary to a Markdown file (`work_log_2025.md`) in a daily/weekly format.

## Tool Usage
- **shell:** `git log`, `git diff --stat` to get raw data.
- **filesystem:** Reading project paths, writing the Markdown report.
- **text-editor:** Appending new entries to the bottom of the log file.

## Memory Architecture
- **Stateless:** The agent is mostly stateless, relying on the `git` history itself as the source of truth.
- **Config File:** Simple JSON file to store paths to watch and regex patterns for categorization.

## Failure Modes
- **Messy Commits:** "wip", "fix stuff" commits produce low-quality logs.
- **Privacy:** Might accidentally log private keys or sensitive customer names if not filtered.
- **Recovery:** User manually edits the generated Markdown file.

## Human Touchpoints
- **Review:** User reads the generated log before sending it to a manager.
- **Configuration:** User defines which repos to track.

## Pros/Cons
- **Pros:** Simple, reliable, privacy-first (local only).
- **Cons:** Dependent on commit message quality; doesn't "understand" the code, just the text descriptions.
