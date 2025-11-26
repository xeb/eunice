# Design 3: The Inverse Conway Architect (Hybrid)

## Purpose
A "Refactoring Agent" that doesn't just warn about mismatches but actively proposes physical code moves to align the codebase with the team structure (or vice versa). It aims to minimize "Cognitive Distance" in the architecture.

## Loop Structure
1.  **Analysis Phase:**
    *   Builds the Socio-Technical Graph (as in Design 2).
    *   Calculates a **"Tension Score"** for every directory: `(External Dependencies) / (Internal Cohesion)`.
2.  **Optimization Phase:**
    *   Runs a clustering algorithm (simulated annealing) to find a "Better File Layout".
    *   Hypothesis: "If `utils/date_formatter.ts` moved to `common/`, the dependency tension would drop by 15%."
3.  **Proposal Phase:**
    *   Creates a "Migration Plan" (Markdown file).
    *   Can optionally scaffold the `git mv` commands.
4.  **Verification:**
    *   Checks if the move breaks imports (using grep/LSP).

## Tool Usage
*   **memory:** For the graph and optimization state.
*   **filesystem:** To verify paths and check for move conflicts.
*   **shell:** `git mv`, build commands to verify moves.
*   **grep:** To find all references that need updating.

## Memory Architecture
*   **Hybrid:**
    *   **Graph:** For the abstract model of dependencies.
    *   **Filesystem:** For the concrete "Proposal" artifacts (Plan A, Plan B).

## Failure Modes
*   **Breaking Builds:** Moving files in modern JS/TS ecosystems (path aliases, tsconfig) is notorious for breaking builds. (Mitigation: Dry-run mode only, or integration with `tsc` to verify).
*   **Politics:** Suggesting "Team A should own this" can be political. (Mitigation: Phrasing proposals as "Architectural Suggestions" based purely on data).

## Human Touchpoints
*   **Negotiation:** The agent posts a "Refactoring Proposal". Humans discuss.
*   **Execution:** Human runs the generated shell script to apply the move.
