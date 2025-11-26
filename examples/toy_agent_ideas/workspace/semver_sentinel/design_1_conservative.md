# Design 1: The "Repo Watchdog" (Conservative)

## Purpose
A background agent that monitors a git repository and ensures that the `CHANGELOG.md` and `package.json` version match the reality of the code changes *before* a human merges a PR. It acts as a CI/CD status check that runs locally or on a server.

## Loop Structure
1.  **Poll for Changes:** Watch for new commits on feature branches or `main`.
2.  **Analyze Diff:** Use `git diff` to see changed files.
3.  **Verify Conventions:** Check if commit messages follow "Conventional Commits".
4.  **Dry Run Versioning:** Calculate what the next version *should* be based on messages.
5.  **Report:** If the `package.json` wasnt updated, or if the messages dont match the code (simple heuristic), post a warning to a local log or PR comment (if integrated).

## Tool Usage
*   **Shell:** `git log`, `git diff`, `npm version`.
*   **Filesystem:** Read `package.json`, `CHANGELOG.md`.
*   **Grep:** Check for patterns in commit messages.

## Memory Architecture
*   **Stateless:** Primarily relies on git history as the source of truth.
*   **Cache:** Might store the "last checked commit" in a simple file to avoid re-processing.

## Failure Modes
*   **False Positives:** Complaining about a missing version bump when one isn't needed (e.g., CI skip).
*   **Convention Fatigue:** Annoying developers by pedantically checking string formats.

## Human Touchpoints
*   **PR Review:** The agent is a passive validator; humans must fix the issues.
*   **Config:** Humans set the rules (e.g., "allow sloppy commits on dev branches").
