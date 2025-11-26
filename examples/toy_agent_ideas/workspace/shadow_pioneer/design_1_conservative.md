# Design 1: The Dependency Scout

## Purpose
To autonomously verify if dependency upgrades are safe, eliminating the manual toil of "bump version, run tests, revert if fail". It turns the maintenance backlog into a "Verified Opportunities" list.

## Loop Structure
1. **Poll:** Check `package.json` against `npm view` (via shell/web) to find outdated packages.
2. **Branch:** Create a git branch `shadow/bump-${package}-${version}`.
3. **Apply:** Run `npm install ${package}@${version}`.
4. **Verify:** Execute the project's test suite (identified via `npm test` or config).
5. **Report:**
   - **Success:** Create a `passed_upgrades.md` report with a one-click command to merge.
   - **Failure:** Log the error output to a `blocked_upgrades.json` file for human review (no fixes attempted).
6. **Cleanup:** Delete the shadow branch immediately to keep repo clean.

## Tool Usage
- **shell:** `git checkout`, `npm install`, `npm test`.
- **filesystem:** Reading `package.json`, writing reports.
- **grep:** Parsing test output for failure reasons.

## Memory Architecture
- **Filesystem-based:** Simple JSON logs (`last_checked.json`, `compatibility_matrix.json`).
- **No Graph needed:** State is simple (Current vs Candidate).

## Failure Modes
- **Infinite Loops:** Upgrading the same failing package repeatedly. *Fix:* Track "Last Attempted" timestamp in JSON.
- **Resource Exhaustion:** Running too many builds. *Fix:* Serial execution with `sleep` intervals.
- **Dirty State:** Leaving uncommitted files. *Fix:* Always run `git reset --hard` before starting.

## Human Touchpoints
- **Review:** User reads `passed_upgrades.md`.
- **Action:** User manually executes the merge command provided.
