# Design 1: The Scripted Critic

## Purpose
To evaluate Developer Experience (DX) by executing standard user stories and quantifying "friction" based on error verbosity and retry counts, rather than just binary pass/fail status.

## Core Loop
1. **Load User Story:** Read a defined sequence of high-level goals (e.g., "Install CLI", "Login", "Deploy App").
2. **Execution:** Attempt to execute the standard commands associated with these goals using `shell`.
3. **Observation:** Captures stdout/stderr.
4. **Friction Analysis:**
   - If a command fails, use `grep` to classify the error (e.g., "Permission denied" vs "Syntax error").
   - Measure "Time to Recovery" if a retry logic is defined.
5. **Reporting:** Generate a `dx_score.md` report.

## Tool Usage
- **shell:** Executes the CLI commands / scripts.
- **grep:** Scans logs for error patterns and warning keywords.
- **filesystem:** Reads the user stories and writes the friction report.

## Memory Architecture
- **Stateless/File-based:** Relies on a config file for user stories and a log file for results. Does not maintain long-term memory across sessions.

## Failure Modes
- **Rigidity:** If the CLI syntax changes slightly (e.g., `flag` becomes `--flag`), the script fails entirely.
- **False Negatives:** Might flag a long-running process as "slow" even if it's normal behavior.

## Human Touchpoints
- **Input:** User must provide the "Happy Path" script.
- **Output:** Read-only report.
