# Final Architecture: The Digital Customer (Frustration Auditor)

## Purpose
An autonomous agent that evaluates the **Developer Experience (DX)** and **User Experience (UX)** of CLI tools and APIs by simulating a user's learning curve. Unlike traditional test runners that check for correctness, this agent checks for *usability*, quantifying "frustration" through metrics like error recovery time, documentation lookup frequency, and command retry rates.

## System Architecture

### 1. The Persona Engine (Core)
The agent adopts a "Persona" with specific knowledge constraints (e.g., "Novice" knows nothing, "Expert" knows aliases).
- **Core Tool:** `memory`
- **Function:** Maintains a "Mental Model" of the tool being tested.
  - *Novice Mode:* Starts with empty memory. Must discover commands via `--help`.
  - *Expert Mode:* Pre-loaded with a graph of commands and flags.

### 2. The Execution Loop
1.  **Goal Ingestion:** "Deploy the app to staging."
2.  **Strategy Formulation:** Queries Memory Graph for a path.
    - *Hit:* Executes command.
    - *Miss:* Enters **Discovery Mode** (runs `help`, reads docs).
3.  **Action:** Uses `shell` to execute.
4.  **Reaction Analysis:**
    - **Success:** Updates Memory Graph (Reinforcement).
    - **Failure:** Analyzes stderr.
      - *Self-Correction:* Tries to infer the fix (e.g., typo correction).
      - *Documentation Search:* Uses `web` to find the error code.
    - **Frustration Event:** Logs a "Frustration Point" if the error was cryptic or the fix required external search.

### 3. The Frustration Metric
The agent generates a `dx_report.md` containing:
- **Confusion Score:** (Help Lookups / Total Commands)
- **Friction Index:** (Error Count * Severity)
- **Time-to-Hello-World:** Wall clock time to complete the onboarding task.

## Tool Usage
- **memory:** Stores the "Mental Model" (Graph of Commands <-> Goals).
- **shell:** Executes commands and captures exit codes/output.
- **web:** Simulates a user Googling for answers (Brave Search).
- **filesystem:** Snapshotting state before/after commands to detect side effects; Logging reports.

## Persistence Strategy
- **Memory Graph:** Persists the "Learned Knowledge" of the tool.
  - *Benefit:* You can see exactly *what* the agent misunderstood. If the agent thinks `deploy` requires `--force` because it failed once without it, that's a valuable insight into potential user misconceptions.
- **Filesystem:** Stores the detailed logs and the final Markdown report.

## Failure Modes & Recovery
- **Infinite Loops:** Agent keeps retrying a failing command. *Mitigation:* "Frustration Threshold" (max 3 retries) triggers a "Give Up" event and moves to the next task.
- **Destructive Exploration:** Agent guesses a command that deletes data. *Mitigation:* Run in a containerized/sandboxed environment; Denylist dangerous keywords.

## Human Touchpoints
- **Goal Definition:** Human sets the high-level goals.
- **Report Review:** Human reviews the "Frustration Log" to identify documentation gaps or confusing error messages.
