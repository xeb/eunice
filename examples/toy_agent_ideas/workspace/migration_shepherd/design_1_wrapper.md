# Design 1: The Code-Level Wrapper (Conservative)

## Purpose
To safely refactor specific functions or classes within a legacy codebase by introducing a temporary "shim" that validates the new implementation against the old one in production/runtime.

## Loop Structure
1. **Identify:** Agent scans for functions marked with `@deprecated` or specific TODO comments (e.g., `// TODO: Refactor this`).
2. **Scaffold:** Agent renames `target_func` to `legacy_target_func`.
3. **Generate:** Agent creates a new `modern_target_func` (or asks user to provide it).
4. **Wrap:** Agent creates a new `target_func` that:
   - Calls `legacy_target_func(args)` -> `result_old`
   - Calls `modern_target_func(args)` -> `result_new`
   - Compares results.
   - Logs mismatches to a file.
   - Returns `result_old`.
5. **Monitor:** Agent periodically reads the log file using `filesystem`.
6. **Report:** If mismatch rate < 1%, Agent suggests "Switchover" to user.

## Tool Usage
- **grep:** Finding targets (`grep -r "@deprecated" .`).
- **filesystem:** Reading source code, writing the wrapper.
- **text-editor:** Applying the rename and insertion of the shim.
- **shell:** Running the build/tests to ensure the shim compiles.

## Memory Architecture
- **State:** Simple file-based logs (`migration_mismatches.log`).
- **Persistence:** None required beyond the filesystem.

## Failure Modes
- **Side Effects:** If the function modifies global state or DB, calling it twice is dangerous.
- **Recovery:** Agent must detect if the function is "pure" before wrapping. If not, it falls back to Design 3 (Test Recorder).

## Human Touchpoints
- User must approve the "Modern" implementation.
- User must approve the final "Switchover".
