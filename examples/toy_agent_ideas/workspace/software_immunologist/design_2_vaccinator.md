# Design 2: The Vaccinator (Autonomous)

## Purpose
The Vaccinator is an active maintenance worker. It doesn't just watch; it experiments. It treats updates as "clinical trials"—isolating them, testing them, and only merging if the "patient" (codebase) survives without symptoms (errors).

## Loop Structure
1. **Identify Candidates:** Scan for outdated packages. Prioritize by security risk (via Web) or age.
2. **Isolation:** Create a git branch `chore/vaccinate-lib-x`.
3. **Inoculation (Update):**
   - Run shell commands to update the package (e.g., `npm install lib@latest`).
   - Use `grep` to find direct usages of the library in the code.
4. **Observation (Test):**
   - Run the project's test suite (`npm test`, `cargo test`).
   - If it fails, capture the error output.
5. **Reaction (Fix/Reject):**
   - **Simple Fix:** If the error is a known pattern (e.g., "import X is now Y"), use `text-editor` to patch the code.
   - **Complex Failure:** If tests still fail, revert the update. Record the failure in `memory`: "Lib X v2.0 broke tests with Error Y".
6. **Conclusion:**
   - If successful: Push PR with "Tested & Verified" badge.
   - If failed: Delete branch, log "Incompatible" in memory.

## Tool Usage
- **shell:** Heavy usage for git, build, test, and install commands.
- **grep:** Locate usage of updated packages to check for deprecations.
- **text-editor:** Apply small fixes (imports, config changes).
- **memory:** Remembers *which* updates failed and *why*, preventing infinite retry loops.

## Memory Architecture
- **Nodes:** `Attempt`, `ErrorPattern`, `PatchStrategy`.
- **Edges:** `CAUSED_ERROR`, `FIXED_BY`, `BLOCKED_UNTIL`.
- **Insight:** The agent learns valid migration paths. If it fixes `Lib A` upgrade in Project 1, it knows how to fix it in Project 2.

## Failure Modes
- **False Negative:** Tests pass, but runtime logic breaks (silent failure).
- **Infinite Loops:** Constantly trying to update a broken package. (Solved by blocking in Memory).
- **Destructive Edit:** `text-editor` mangles code. (Solved by Git reset).

## Human Touchpoints
- **PR Review:** Humans must merge the final PR. The agent does the grunt work of "getting it to green".
