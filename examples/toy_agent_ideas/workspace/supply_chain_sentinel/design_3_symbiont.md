# Design 3: The Health Symbiont (Hybrid/Autonomous)

## Purpose
An active participant in the development lifecycle. It doesn't just report risk; it **mitigates** it. It acts as a "Dependency Manager" that is also a "QA Engineer" and "Researcher".

## Loop Structure
1.  **Monitor & Intercept**:
    *   Watches for `package.json` changes or new upstream versions.
2.  **The "Sandbox" Test**:
    *   When an update is available, it clones the repo to a shadow directory.
    *   Applies the update.
    *   Runs `shell` test commands.
3.  **The "fixer" Loop**:
    *   If tests fail, it uses `grep` to isolate the error and `web` to search for "breaking changes".
    *   It attempts to *patch* the code using `text-editor` (e.g., renaming a function call).
    *   If successful, it prepares a Pull Request (via `git` shell commands).
4.  **Proactive Replacement**:
    *   If a package is identified as "High Risk" (Design 2 logic), it actively searches for **Alternatives**.
    *   It generates a "Migration Strategy" document: "Replace `request` with `axios`. Here is the cost/benefit analysis."

## Tool Usage
*   **shell**: Run tests, git operations, package installations.
*   **text-editor**: Apply patches and migration fixes.
*   **memory**: Stores "Knowledge of Incompatibilities" (e.g., "Lib A v2.0 breaks Lib B v1.5").
*   **web**: Search for changelogs and migration guides.

## Memory Architecture
*   **Knowledge Graph**: Stores *Rules* and *Events* rather than just static topology.
    *   `(Event:UpdateFail)-[CAUSED_BY]->(BreakingChange:API_Rename)`
    *   `(Package:Legacy)-[HAS_ALTERNATIVE]->(Package:Modern)`

## Failure Modes
*   **Destructive Edits**: The agent might introduce subtle bugs while trying to patch code.
*   **Test Flakiness**: Might discard good updates due to flaky tests.
*   **Resource Intensity**: Running shadow builds consumes CPU/Disk.

## Human Touchpoints
*   **Pull Requests**: All actions result in a PR/Branch. The human must merge.
*   **Configuration**: User sets "Autonomy Level" (e.g., "Auto-update minor versions", "Ask for major").
