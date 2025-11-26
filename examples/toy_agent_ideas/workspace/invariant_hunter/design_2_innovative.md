# Design 2: The Autonomous Property Tester

## Purpose
An active agent that converts natural language constraints into executable property-based tests (PBT). It doesn't just find gaps; it fills them by writing regression tests that prove or disprove the constraints.

## Loop Structure
1. **Mining**: Scans codebase for constraints (like Design 1).
2. **Synthesis**:
   - Selects a high-confidence "Phantom Constraint" (Documented but untested).
   - Generates a PBT using a framework (e.g., `fast-check` for TS, `hypothesis` for Python).
   - Uses `text-editor` to create a temporary test file.
3. **Execution**:
   - Runs the test using `shell`.
   - **Case A (Pass)**: The constraint holds. The agent proposes adding this test permanently.
   - **Case B (Fail)**: The constraint is false (code behaves differently). The agent reports a "Documentation/Code Drift".
4. **Refinement**:
   - If the test fails due to syntax/import errors, the agent self-corrects the test code (up to 3 retries).
   - Uses `web` to look up library docs if the PBT framework usage is incorrect.

## Tool Usage
- `filesystem`: Reading code to understand types/imports.
- `text-editor`: Writing and modifying test files.
- `shell`: Running the test runner (e.g., `npm test`).
- `memory`: Tracking which constraints have been "proven" vs "disproven".

## Memory Architecture
- **State**: `PROVEN`, `DISPROVEN`, `FLAKY`, `SYNTAX_ERROR`.
- **Graph**: Links `Constraint` nodes to `TestFile` paths.

## Failure Modes
- **Infinite Loops**: The agent keeps trying to write a test that fails to compile. (Mitigation: Retry limits).
- **Destructive Tests**: Generating tests that have side effects (e.g., deleting real DB rows). (Mitigation: Strict sandbox or mock-only mandate).
- **Hallucinated APIs**: Using non-existent functions. (Mitigation: `grep` to verify function signatures before usage).

## Human Touchpoints
- **Approval**: The agent creates a "Pending Tests" folder. It never modifies existing test files directly.
- **Review**: User moves files from `pending/` to `tests/` to accept them.
