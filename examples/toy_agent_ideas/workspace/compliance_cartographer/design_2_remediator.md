# Design 2: The License Auto-Remediator

## Purpose
An active agent that not only finds violations but actively attempts to fix them by finding compliant alternatives or "forking and fixing" the dependency.

## Loop Structure
1. **Monitor:** Watch for `git push` or file changes in manifests.
2. **Detect:** Identify a "Banned License" (e.g., WTFPL) in the dependency tree.
3. **Strategize:**
   - **Strategy A (Swap):** Search web for "alternatives to [lib] with MIT license".
   - **Strategy B (Isolate):** Suggest moving the dependency to a separate microservice to isolate the viral license.
4. **Execute:**
   - Create a new branch.
   - Uninstall bad lib, install good lib.
   - Run `grep` to find usage and attempt API translation (using simple regex or LLM inference).
   - Run tests.
5. **Propose:** Submit a PR with the substitution and test results.

## Tool Usage
- **web:** Search for "lightweight alternative to moment.js", "library comparison".
- **shell:** `npm uninstall X`, `npm install Y`, `npm test`.
- **filesystem:** Edit code to replace import statements.
- **memory:** Track "Attempted Fixes" to avoid retrying failed swaps.

## Memory Architecture
- **Entities:** `Vulnerability`, `Solution`, `API_Signature`.
- **Relations:** `Lib_A IS_ALTERNATIVE_TO Lib_B`, `Lib_A COMPATIBILITY_SCORE High`.

## Failure Modes
- **API Mismatch:** Replacement library breaks the build (Agent reverts and flags as "Manual Fix Required").
- **Subtle Bugs:** Tests pass but behavior changes (Risk inherent to auto-refactoring).

## Human Touchpoints
- Merging the PR.
- Defining the "Banned List".
