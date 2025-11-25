# Design 2: The Semantic Gardener (Experimental)

## Purpose
To treat code quality as a dynamic ecosystem, autonomously identifying "rotting" code (high complexity + high churn) and proactively restructuring it to reduce cognitive load.

## Core Toolset
- **memory:** To store a "Health Graph" of the codebase (Functions, Classes, Modules) and their "Debt Score".
- **shell:** To run complexity analysis tools (Cyclomatic complexity, Halstead metrics).
- **grep/text-editor:** To analyze call graphs and perform AST-based transformations.
- **filesystem:** To track git history/churn.

## Loop Structure
1. **Surveillance (Background):**
   - Continuously monitor `git log` to identify "Hotspots" (files changed frequently).
   - Run complexity analysis on these hotspots.
2. **Scoring:**
   - Update the Memory Graph: `Entity(FunctionX) -> hasMetric(Complexity: 15) -> hasObservation(Changed 10 times in last week)`.
   - Calculate "Refactor Priority" = Complexity × Churn Rate.
3. **Planning:**
   - Select the highest priority entity.
   - Generate a refactoring strategy: "Extract Method", "Introduce Parameter Object", "Split Class".
4. **Execution:**
   - **Isolate:** Create a temporary branch.
   - **Transform:** Use LLM logic + `text-editor` to rewrite the code.
   - **Verify:** Run existing tests. If coverage is low, *generate new tests first* (using the existing behavior as the oracle).
5. **Proposal:**
   - Open a PR with a detailed "Why" explanation based on the Memory Graph data ("This function changes every 2 days and has complexity 25").

## Memory Architecture
- **Entities:** `File`, `Class`, `Function`, `Module`.
- **Relations:** `calls`, `imports`, `is_part_of`.
- **Observations:** Historical churn, bug frequency, "smell" detection timestamps.
- **Goal:** To remember *why* code is the way it is. If a human rejects a refactor, store that preference ("User rejected splitting UserAuth class").

## Failure Modes
- **Regression:** Refactoring introduces subtle bugs. Mitigation: Enforce strict "Green-Red-Green" test cycles.
- **Context Loss:** Breaking code that relies on implicit state/side effects. Mitigation: Analyze variable scope deeply before extraction.

## Human Touchpoints
- **Approval:** All changes result in PRs. The agent never pushes directly to main.
- **Feedback:** Humans can comment "ignore this module", which updates the Memory Graph.

## Pros & Cons
- **Pros:** Addresses the root cause of debt (complexity + churn), not just style. Improves long-term velocity.
- **Cons:** High risk of "breaking changes" or creating code that feels "alien" to the original author. Expensive inference.
