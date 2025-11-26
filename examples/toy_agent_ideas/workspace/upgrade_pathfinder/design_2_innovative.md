# Design 2: The Shadow Upgrader (Innovative)

## Purpose
To autonomously attempt dependency upgrades in an isolated "shadow" environment, proving that an upgrade is safe (or fixing it until it is) before presenting it to the human. It shifts the burden of "upgrade anxiety" from human to machine.

## Loop Structure
1. **Selection:** Identifies one outdated dependency (e.g., lodash v3 -> v4).
2. **Isolation:**
   - Creates a temporary directory.
   - Copies the project (excluding .git metadata but keeping .gitignore rules).
3. **Attempt:**
   - Runs the upgrade command (npm install lodash@latest).
   - Runs the project's test suite (npm test).
4. **Iterate (The "Fixer" Loop):**
   - If tests fail, it parses the error output.
   - Uses grep to find the code causing the error.
   - Uses text-editor to apply heuristic fixes (e.g., renaming a deprecated function call found in the migration guide).
   - Re-runs tests (max N retries).
5. **Deliver:**
   - If tests pass, generates a .patch file or a Pull Request description with the changelog.
   - If tests fail after retries, logs a "Blocker Report" explaining exactly why the upgrade failed.

## Tool Usage
- **shell:** Heavy usage. cp, git, npm/pip, pytest/jest.
- **text-editor:** text-editor_edit_text_file_contents to modify code based on test failures.
- **web:** web_brave_web_search to find "Migration Guide from X to Y" and specific error messages.
- **filesystem:** Managing the temporary workspace.

## Memory Architecture
- **Short-term:** Context of the current upgrade attempt (errors, fixes tried).
- **Long-term:** "Blocker Graph". Library X is blocked by File Y due to Error Z. This prevents retrying known impossible upgrades repeatedly.

## Failure Modes
- **Infinite Loop:** Trying to fix the same error endlessly. -> Recovery: Strict max-retry limit.
- **Side Effects:** Tests that interact with real DBs/APIs (if not mocked). -> Recovery: User must flag "safe" test commands.
- **Disk Space:** Shadow copies consuming space. -> Recovery: Aggressive cleanup after each loop.

## Human Touchpoints
- **Review:** The agent provides a "Pre-validated Patch". The human just needs to review and merge.
