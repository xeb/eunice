# Design 2: The Adaptive Persona

## Purpose
To simulate a new user learning the software from scratch without a script, measuring the "Intuitive Gap"—the difference between how the software works and how a user *guesses* it works.

## Core Loop
1. **Goal Setting:** Agent is given a high-level goal (e.g., "Deploy the app in the current folder").
2. **Exploration:**
   - Runs `help` or `--help` using `shell`.
   - Uses `memory` to store a graph of discovered Commands, Flags, and their descriptions.
3. **Planning:**
   - Queries `memory` to find the most likely command sequence to achieve the goal.
   - *Example:* "Goal: Deploy" -> Memory contains `deploy` command? Yes -> Execute.
4. **Execution & Correction:**
   - If execution fails, reads the error message.
   - If the error suggests a fix (e.g., "Did you mean `deploy --force`?"), it updates `memory` and retries.
   - If no fix is suggested, it searches `web` (documentation) for the specific error.
5. **Metric: Confusion Score:**
   - Score = (Number of failed attempts) + (Number of documentation lookups).

## Tool Usage
- **memory:** Stores the "Mental Model" of the CLI (Nodes: Commands, Edges: Subcommands/Flags).
- **shell:** Interacts with the software.
- **web:** Searches documentation when stuck.
- **filesystem:** Logs the session trace.

## Memory Architecture
- **Graph-based:**
  - **Entity:** `Command` (e.g., `npm install`)
  - **Observation:** `Usage string`, `Success rate`, `Related tasks`.
  - **Relation:** `HAS_SUBCOMMAND`, `REQUIRES_FLAG`.

## Failure Modes
- **Rabbit Holes:** Agent might get stuck exploring irrelevant help sub-pages.
- **Destructive Actions:** Without a sandbox, "exploring" might lead to `delete --all` if the agent guesses wrong.

## Human Touchpoints
- **Safety Bounds:** User defines "Forbidden Commands" (e.g., `rm`, `drop`).
- **Goal Definition:** User sets the initial objective.
