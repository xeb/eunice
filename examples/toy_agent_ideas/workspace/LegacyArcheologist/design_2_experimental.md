# Design 2: The Refactoring Sandboxer (Experimental)

## Purpose
An active intervention agent that doesn't just read code but attempts to modernize it. It operates on the principle of "Refactoring by Sandbox," where changes are proven safe in isolation before being proposed.

## Core Toolset
* **filesystem**: For creating sandbox environments and copying files.
* **shell**: For executing build commands, test runners, and linters.
* **text-editor**: For applying code transformations and patches.

## Loop Structure
1. **Target Identification**: Scans codebase for "smells" (e.g., long functions, deprecated syntax) using `grep`.
2. **Isolation**:
   - Creates a `sandbox/` directory.
   - Copies the target file and its immediate dependencies.
   - Stubs out external dependencies if necessary.
3. **Characterization**:
   - Generates a "characterization test" (a test that asserts the current behavior, even if buggy).
   - Runs the test via `shell` to confirm it passes (capturing the baseline).
4. **Refactoring**:
   - Uses `text-editor` to apply modernization (e.g., `var` -> `let/const`, converting loops to maps).
   - Reruns the characterization test.
5. **Proposal**:
   - If tests pass, generates a diff/patch.
   - Writes a rationale document explaining the change.

## Memory Architecture
* **Entities**: `RefactoringCandidate`, `TestResult`, `Patch`.
* **Observations**: Success rate of previous refactoring strategies.
* **Relations**: `COVERED_BY` (File -> Test), `TRANSFORMED_INTO` (OldCode -> NewCode).

## Failure Modes & Recovery
* **Build Breakage**: Sandbox environment might lack build context. *Mitigation*: Fallback to "Lint-only" improvements if full build fails.
* **Behavior Drift**: Tests might not cover edge cases. *Mitigation*: User review is mandatory before applying patches to the main codebase.

## Human Touchpoints
* **Gatekeeper**: The agent **cannot** modify the main codebase directly. It only produces `.patch` files or Pull Requests.
* **Feedback**: Humans vote on refactorings; the agent learns which styles are preferred.

