# Final Design: The Entropy Gardener

## Purpose
The Entropy Gardener is an autonomous QA agent that strengthens code by systematically attacking it. It unifies **Mutation Testing** (to find weak tests) and **Smart Fuzzing** (to find weak code) into a single, self-reinforcing loop. Its goal is to maintain a "Living Regression Suite" that evolves faster than the codebase itself.

## Core Loop: The "Weed & Seed" Cycle

1.  **Surveillance (Trigger):**
    *   Watches for file changes via `filesystem`.
    *   Identifies modified functions and their associated tests.

2.  **Phase 1: Weeding (Mutation Testing)**
    *   The agent generates "Mutants" of the new code (e.g., swapping `>` with `>=`, returning `null`).
    *   It runs existing tests.
    *   **Outcome:** If tests pass (Mutant Survives), the agent identifies a "Test Gap."

3.  **Phase 2: Seeding (Targeted Fuzzing)**
    *   For every "Test Gap," the agent analyzes the function signature and documentation.
    *   It uses LLM capabilities to generate "Hostile Inputs" specifically designed to exploit the logic that the mutant revealed (e.g., if `x > 0` was mutated to `x >= 0` and survived, it fuzzes around `0`).
    *   It runs the function with these inputs in a sandbox.

4.  **Phase 3: Harvest (Test Generation)**
    *   If a Fuzz Input causes a crash OR kills the Surviving Mutant, it is "Harvested."
    *   The agent writes a new, permanent unit test file (e.g., `test_user_edge_cases.py`) containing this input.

5.  **Persistence:**
    *   The "Mutation Score" and "Crash History" are stored in the Memory Graph.

## Tool Usage
*   **shell:** Execute tests (`npm test`, `pytest`), manage git branches (to isolate experiments), run sandboxed scripts.
*   **filesystem:** Read code, write temporary mutants, write permanent new tests.
*   **grep:** Locate function definitions and call sites to determine attack surface.
*   **memory:**
    *   Store `Function -> MutationScore` (to track code health over time).
    *   Store `InputPattern -> SuccessRate` (to learn which fuzz patterns work best for this specific project).

## Memory Architecture
*   **Nodes:** `Function`, `Mutant`, `FuzzPattern`, `TestGap`
*   **Relations:**
    *   `(Mutant) EXPOSES (TestGap)`
    *   `(FuzzPattern) FILLS (TestGap)`
    *   `(Function) HAS_VULNERABILITY (CrashType)`

## Failure Modes & Recovery
1.  **Infinite Loops (The "Hanging Garden"):** Mutating loop conditions can freeze the process.
    *   *Fix:* All shell executions have strict timeouts (e.g., `timeout 5s pytest`).
2.  **Destructive Fuzzing:** Fuzzing a DB-write function might wipe data.
    *   *Fix:* The agent only fuzzes in a strictly mocked environment or requires a `@fuzzable` annotation on functions.
3.  **Noise:** Generating thousands of trivial tests.
    *   *Fix:* Only save tests that kill a *previously surviving mutant* or cause a crash. Deduplicate tests based on code coverage.

## Human Touchpoints
*   **The "Compost Heap":** A generated Markdown report showing "Mutants that survived" (Tests you need to write).
*   **Pull Request Integration:** The agent can comment on PRs: "This change lowered the Mutation Score by 5%. Here is a test case that breaks your new logic."
