# Design 3: The Counterfactual Simulator (Technical)

## Purpose
To autonomously explore the "search space" of potential code changes by simulating thousands of "what-if" scenarios (mutations) in a sandboxed environment, selecting only those that pass rigorous tests.

## Loop Structure
1.  **Observation**:
    *   Agent watches for a failing test or a specific `@dream` annotation in the code.
2.  **Dream State (Simulation)**:
    *   **Clone**: Copies the codebase to a temporary location `/tmp/dream_sandbox`.
    *   **Mutate**: Applies AST-based transformations to the target function (e.g., swap conditionals, change variable types, reorder operations).
    *   **Evolution**:
        *   Generation 1: Random mutations.
        *   Test: Run project test suite.
        *   Select: Keep survivors (those that pass more tests or crash less).
        *   Crossover: Combine changes from survivors.
3.  **Lucidity (Verification)**:
    *   If a mutation passes ALL tests, it is flagged as a "Prophetic Dream".
    *   Agent attempts to "explain" *why* it works (reverse engineering the mutation).
4.  **Wake Up**:
    *   Presents a Git Patch file `dreams/fix_candidate.patch`.

## Tool Usage
*   **filesystem**: Heavy usage (copying, writing code, running tests).
*   **shell**: Executing build/test commands.
*   **grep**: Locating the `@dream` target.
*   **memory**: Tracking "Evolutionary Lineage" of the patches (Gen 1 -> Gen 2).

## Memory Architecture
*   **Graph**: Evolutionary Tree.
    *   Entities: `Mutation`, `TestResult`.
    *   Relations: `parent_of`, `passed_test`.
*   **Persistence**: Ephemeral. Cleared after the "Dream" ends, except for the winner.

## Failure Modes
*   **Infinite Loops**: Mutations cause hangs. (Mitigation: Strict timeouts in shell commands).
*   **Destructive Acts**: Code deletes files. (Mitigation: Sandbox / Containerization).
*   **Overfitting**: Fixes the test but breaks logic not covered by tests. (Mitigation: User review required).

## Human Touchpoints
*   **Gatekeeper**: User must manually apply the patch. The agent never commits directly.
