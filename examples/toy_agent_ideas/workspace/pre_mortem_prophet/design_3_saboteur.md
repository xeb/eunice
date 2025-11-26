# Design 3: The Saboteur (Hybrid/Active)

## Purpose
A "Red Team" agent that doesn't just imagine failure but proves it. It identifies weak points (like Design 1) and generates the *exact* unit test or load script required to break the system (like Chaos Engineering), but does so in a "Sandbox" environment. It provides the "Poison Pill" input that triggers the bug.

## Loop Structure
1.  **Vulnerability Hypothesis**: "I bet this regex in validation.ts has catastrophic backtracking."
2.  **Proof Generation**: Write a test case reproduce_crash.test.ts with the malicious input.
3.  **Execution (Safe Mode)**: Run the test in isolation (using shell).
4.  **Verification**: If it crashes/times out, record the "Confirmed Kill".
5.  **Documentation**: Write the PRE_MORTEM.md explaining exactly how the system *can* be broken right now.

## Tool Usage
*   **shell**: Execute generated tests/scripts.
*   **grep**: Find "smelly" code patterns (regex, recursive functions, unbounded loops).
*   **filesystem**: Write the "Proof of Failure" scripts.

## Memory Architecture
*   **Entities**: Hypothesis, Experiment, Result.
*   **Relations**: REFUTES, CONFIRMS.
*   **Learning**: Remember which types of attacks worked on this codebase to optimize future attempts.

## Failure Modes
*   **Destruction**: Accidentally running the test against Prod (requires strict sandbox barriers).
*   **Resource Exhaustion**: The "test" is an infinite loop that crashes the agent's own machine.

## Human Touchpoints
*   **Permission**: Explicit confirmation before running any "attack" script.
*   **Review**: Humans review the "Confirmed Kills" and prioritize fixes.
