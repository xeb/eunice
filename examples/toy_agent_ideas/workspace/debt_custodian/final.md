# Final Design: The Code Custodian

## Synthesis
After evaluating the conservative "Janitor", the experimental "Architect", and the hybrid "Gardener", the optimal design for a practical, valuable, and safe autonomous agent is a layered approach. We combine the safety of the Janitor with the insight of the Gardener, while avoiding the high-risk autonomous restructuring of the Architect.

## Core Philosophy
**"Trusted automated hygiene, intelligent human-guided structural improvement."**

## Architecture

### Layer 1: The Autonomic Nervous System (Automated Hygiene)
*   **Frequency**: High (On every commit/push).
*   **Scope**: Formatting, linting, import sorting, dead code removal.
*   **Autonomy**: Full. It commits directly to fix branches or the PR branch if checks pass.
*   **Tools**: shell (executing `ruff`, `prettier`, `cargo clippy --fix`), text-editor (simple regex replacements).

### Layer 2: The Cortex (Debt Analysis & Planning)
*   **Frequency**: Medium (Nightly or Weekly).
*   **Scope**: Complexity analysis, dependency cycles, test coverage trends.
*   **Autonomy**: Observational. It does not change code. It updates the **Memory Graph**.
*   **Action**: 
    1.  Calculates a "Debt Score" for each module.
    2.  If a score spikes (e.g., "MainController.ts grew 20% in 2 days"), it generates a **Refactoring Proposal**.
    3.  The proposal is a Markdown file added to a specific `docs/refactoring_proposals/` directory or a Comment on the relevant PR.

### Layer 3: The Surgeon (Interactive Refactoring)
*   **Frequency**: On Demand (Triggered by user).
*   **Scope**: Structural changes (Extract Class, Move Module).
*   **Autonomy**: Human-in-the-loop. The user approves a Proposal from Layer 2.
*   **Action**: The agent executes the plan using `text-editor`, runs tests, and pauses for review.

## Memory Strategy (The Health Graph)
We use the Memory MCP to persist the *trajectory* of the codebase.
*   **Entities**: `File`, `Module`, `Author`.
*   **Observations**: 
    *   "2023-10-27: File X complexity increased by 5."
    *   "2023-10-28: File X failed tests 3 times."
*   **Insight**: This allows the agent to say "This file is unstable" rather than just "This file is large."

## The "Loop" Implementation
```bash
while true; do
  # 1. Pull latest changes
  git pull origin main

  # 2. Layer 1: Hygiene
  run_linters_and_fix
  if [ changes_made ]; then
    run_tests && git commit -am "chore: auto-hygiene"
  fi

  # 3. Layer 2: Analysis
  update_memory_graph # Scans files, updates complexity scores in Memory
  
  # 4. Check for Spikes
  bad_files=$(query_memory_for_spikes)
  for file in $bad_files; do
    generate_proposal $file # Writes to docs/proposals/
  done

  # 5. Sleep
  sleep 6h
done
```

## Safety & Recovery
*   **Sandbox**: All invasive edits happen in a temporary git branch.
*   **Test Gate**: No commit is pushed without a passing test suite.
*   **Revertibility**: The Memory Graph tracks what the agent did. If a refactor goes bad, the agent can "undo" by reverting the specific git commit associated with that Memory action ID.

## Conclusion
The Code Custodian provides immediate value through automated hygiene while building a long-term strategic asset (the Debt Graph) that empowers developers to make informed architectural decisions, rather than trying to replace the architect entirely.
