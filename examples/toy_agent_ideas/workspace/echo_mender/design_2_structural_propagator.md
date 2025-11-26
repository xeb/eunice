# Design 2: The Structural Propagator (Innovative)

## Purpose
To autonomously infer abstract "fix patterns" from developer commits and apply them to semantically similar code, treating the codebase as a self-healing organism.

## Loop Structure
1.  **Learning:** On every commit, the agent analyzes the diff. It uses `memory` to query if this matches a known "Vulnerability Signature".
2.  **Abstraction:** If it's a new fix, it generalizes the diff into a **structural search pattern**.
    *   *Example:* `foo.bar(x)` -> `foo.baz(x, true)` becomes `$A.bar($B) -> $A.baz($B, true)`.
3.  **Discovery:** It runs this structural search (simulated via regex or AST parsing tools if available) against the codebase.
4.  **Patching:** For every match, it creates a new "Propagated Fix" branch and applies the patch using `text-editor`.
5.  **Verification:** It runs local tests. If they pass, it opens a PR (or creates a patch file).

## Tool Usage
*   `memory`: Stores the library of "Fix Patterns" (Pattern Graph).
    *   Nodes: `Pattern`, `CommitHash`, `ImpactScore`.
    *   Edges: `FIXES`, `DERIVED_FROM`.
*   `grep`: Used for finding pattern candidates.
*   `text-editor`: Used to apply the propagated fixes safely.
*   `shell`: To manage git branches and run tests.

## Memory Architecture
*   **The Immune System:** A persistent graph of all past fixes. Over time, this becomes a database of "anti-patterns" specific to this project.
*   **Deduplication:** Prevents re-applying the same fix or suggesting fixes that were previously rejected.

## Failure Modes
*   **Over-Generalization:** A pattern might be too broad and break correct code.
*   **Mitigation:** The agent requires a "Confidence Score" (based on pattern complexity) before acting. It also relies strictly on the test suite.

## Human Touchpoints
*   **Gatekeeper:** The agent prepares the work but waits for a "Merge" command or PR approval.
