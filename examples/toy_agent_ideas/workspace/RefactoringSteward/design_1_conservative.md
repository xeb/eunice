# Design 1: The Linter-Enforcer (Conservative)

## Purpose
To automatically enforce code style and perform safe, rule-based micro-refactorings without human intervention, ensuring the codebase remains compliant with defined standards.

## Core Toolset
- **shell:** To execute existing linters (ESLint, Prettier, Ruff, Black, Cargo fmt) and run tests.
- **filesystem:** To discover files and read configuration.
- **text-editor:** To apply specific patches if auto-fixers fail or for custom rule application.
- **grep:** To find patterns that standard linters might miss (e.g., TODO comments, deprecated API usage).

## Loop Structure
1. **Discovery:**
   - Scan the filesystem for changed files (git status) or run a full sweep.
   - Identify file types to determine applicable tools.
2. **Analysis:**
   - Run linter commands with `--dry-run` or json output to identify violations.
   - Use `grep` to check for forbidden patterns (e.g., `console.log` in production, hardcoded secrets).
3. **Remediation:**
   - **Phase A (Safe):** Run linter `--fix` commands (e.g., `eslint --fix`).
   - **Phase B (Custom):** If specific issues remain (e.g., "replace deprecated function X with Y"), use `text-editor` to apply precise regex-based patches.
4. **Verification:**
   - Run project unit tests. If tests fail, **revert** changes immediately.
5. **Commit:**
   - Create a commit with a standardized message: `chore(lint): auto-fix style violations`.

## Memory Architecture
- **Stateless:** This agent is primarily stateless. It relies on the current state of the filesystem and the configuration files (`.eslintrc`, `pyproject.toml`).
- **Filesystem-as-Memory:** It respects `.ignore` files to know what *not* to touch.

## Failure Modes
- **Broken Build:** If auto-fix breaks the build -> Revert and log the error to `refactor_errors.log`.
- **Infinite Loop:** Two linters fighting (e.g., Prettier vs ESLint) -> Detect if file changes oscillate between two states and stop touching that file.

## Human Touchpoints
- **Configuration:** Humans define the rules in standard config files.
- **PR Review:** The agent pushes to a branch; humans merge.

## Pros & Cons
- **Pros:** Extremely safe, low hallucination risk, integrates with existing ecosystem.
- **Cons:** Limited to stylistic/syntax changes. Cannot perform structural refactoring (extract method, introduce pattern).
