# Design 3: The Active Surveyor (Hybrid)

## Purpose
A proactive agent that not only documents code but actively validates its understanding by writing and running temporary "survey tests" or logging probes. It bridges the gap between static analysis and runtime behavior.

## Loop Structure
**Trigger:** User request ("How does the auth flow work?") or nightly deep-scan.
1. **Hypothesize:** Analyzes code to form a hypothesis (e.g., "I think function X calls function Y").
2. **Probe:**
   - Writes a temporary unit test or inserts a logging statement (using `text-editor`) to verify the hypothesis.
   - Runs the code/test (using `shell`).
3. **Observe Result:** Captures output/logs.
4. **Refine:**
   - If confirmed: Updates the documentation/graph with "Verified" status.
   - If failed: Marks the path as "Dead Code" or "Misunderstood".
5. **Clean Up:** Reverts the code changes (removes probes).
6. **Report:** Generates a "Verified Architecture" report.

## Tool Usage
- **filesystem & text-editor:** Modifying code to inject probes/tests.
- **shell:** Running compilers, test runners, or scripts.
- **memory:** Storing hypotheses and verification results.
- **grep:** Locating injection points.

## Memory Architecture
- **Hybrid:**
  - **Short-term:** File modifications (reverted after use).
  - **Long-term:** Memory graph storing "Verified Facts" vs "Unverified Static Analysis".

## Failure Modes
- **Production Breakage:** If used in a live environment, probes could cause side effects. (Mitigation: strict sandbox/CI-only mode).
- **Infinite Loops:** The agent keeps probing the same failing logic.
- **Recovery:** Hard reset via `git checkout .` to remove all probes.

## Human Touchpoints
- **Approval:** Agent requests permission before running any executable code or modifying files.
- **Sandbox Definition:** User must define the safe execution command (e.g., `npm test`).
