# Design 2: The Counterfactual Graph

## Purpose
To explore the "State Space" of the codebase by proactively simulating significant architectural changes (e.g., "Switch to ESM", "Use Strict Mode", "Upgrade Node Version") and maintaining a persistent map of which futures are viable.

## Loop Structure
1. **Hypothesize:**
   - Agent generates "Hypotheses" (nodes in Memory Graph) based on detected patterns (e.g., "Found `var`; hypothesis: `Convert to let/const`").
   - Agent finds external "Recipes" via Web Search (e.g., "Migration guide for Library X v2").
2. **Simulate:**
   - Spawns a background process for a Shadow Branch.
   - Applies aggressive automated codemods (using `sed` or simple `text-editor` replacements).
3. **Validate:**
   - Runs not just tests, but **Benchmarks** and **Linters**.
   - Captures "Diff Metrics" (Lines saved, Bundle size change).
4. **Persist:**
   - Updates the Memory Graph:
     - `CurrentState --[codemod]--> FutureState (Status: Verified, Gain: -200kb)`
     - `CurrentState --[upgrade]--> FutureState (Status: Failed, Reason: "Syntax Error in file.js")`
5. **Surface:**
   - When the user asks "Can we upgrade?", the agent queries the Graph: "Yes, verified 2 hours ago. 3 minor test failures in module Z."

## Tool Usage
- **memory:** Storing the "Multiverse" of code states.
- **shell:** Running codemods, benchmarks, tests in parallel.
- **web:** Finding migration codemods/scripts.
- **filesystem:** Deep analysis of structure to generate hypotheses.

## Memory Architecture
- **Graph Nodes:** `State` (Attributes: commit_hash, dependencies, config).
- **Graph Edges:** `Transition` (Attributes: patch_content, success_probability, test_result).

## Failure Modes
- **State Explosion:** Trying too many permutations. *Fix:* Limit exploration depth to 1 hop from HEAD.
- **False Positives:** Tests passing but feature broken. *Fix:* Only trust existing coverage; explicitly warn about low coverage areas.

## Human Touchpoints
- **Query:** User asks "Explore migration to X".
- **Notification:** Agent pings: "Did you know we can drop dependency Y with zero side effects?"
