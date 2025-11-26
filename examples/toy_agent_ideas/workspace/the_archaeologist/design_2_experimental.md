# Design 2: The Excavator (Experimental/Autonomous)

## Purpose
To actively reduce technical debt by autonomously finding "safe" refactoring targets (like dead code or unused imports) and removing them, or by upgrading deprecated patterns. It operates on a "high confidence" threshold.

## Loop Structure
1. **Identify:** Query the Memory Graph for "safe" targets (e.g., private functions with 0 callers).
2. **Verify (Pre-flight):** 
   - Run `grep` to triple-check no usages exist (including dynamic strings if possible).
   - Run existing test suite (`shell_execute_command`).
3. **Excavate (Edit):**
   - Use `text-editor` to remove the code or apply the standard fix (e.g., `var` -> `const`).
4. **Verify (Post-flight):**
   - Run tests again.
   - If tests fail -> **Revert** immediately and mark entity as "Load Bearing" in Memory.
   - If tests pass -> **Commit** (or create patch).
5. **Log:** Write to `excavation_log.md`.

## Tool Usage
- **memory:** To query for candidates (e.g., `observation: "unused"`).
- **shell:** To run project tests (e.g., `npm test`, `pytest`).
- **text-editor:** `patch_text_file_contents` to perform surgical edits.
- **filesystem:** To manage backups/restore if reversion is needed.

## Memory Architecture
- **State Tracking:** Entities have states: `Suspected_Dead`, `Confirmed_Dead`, `Load_Bearing`, `Refactored`.
- **Learning:** If a "dead" code removal breaks tests, the agent adds an observation: "Dynamically accessed via reflection" to prevent future attempts.

## Failure Modes
- **Hidden Dependencies:** Reflection or dynamic imports that grep misses.
  - *Recovery:* Strict "Revert on Red" policy. 
- **Drift:** The codebase changes while the agent is "thinking".
  - *Recovery:* Hash-based locking in `text-editor` tool prevents editing stale files.

## Human Touchpoints
- **Review:** The agent works on a separate git branch. Humans review PRs.
- **Emergency Stop:** A "stop" file in the workspace halts execution.
