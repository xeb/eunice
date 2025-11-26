# Design 1: The Janitor (Conservative)

## Purpose
A safe, deterministic background agent focused on enforcing code hygiene and reducing low-level technical debt without altering business logic. It operates strictly on "Safe Refactoring" principles—changes that are guaranteed not to break compilation or tests if the tool is correct.

## Core Loop
1. **Trigger**: Scheduled (nightly) or Triggered (post-merge).
2. **Scan**: Run static analysis tools (e.g., ruff, eslint, cargo clippy) to find violations.
3. **Filter**: Select violations marked as "auto-fixable" or "low-risk" (formatting, unused imports, dead code).
4. **Act**: Apply standard fixes using CLI tools or simple text replacements.
5. **Verify**: Run build and unit tests.
   - If Pass: Commit changes to a new branch janitor/fix-TIMESTAMP.
   - If Fail: Revert changes and log the failure.
6. **Report**: Create a Pull Request with a summary of fixed items.

## Tool Usage
*   **shell**: Primary tool. Executes standard linters, formatters, and git commands.
*   **grep**: Used to count occurrences of specific patterns to track metrics.
*   **filesystem**: Reads config files to understand project standards.

## Memory Architecture
*   **Stateless**: The Janitor does not maintain long-term state between runs. It relies entirely on the current state of the codebase.

## Failure Modes
*   **Build Breakage**: Mitigated by running tests immediately after edits.
*   **Merge Conflicts**: Mitigated by running on fresh main and keeping branches short-lived.
*   **False Positives**: Relies on the configuration of the underlying linting tools.

## Human Touchpoints
*   **PR Review**: All changes go through standard PR review.
*   **Configuration**: Humans configure the linter rules.
