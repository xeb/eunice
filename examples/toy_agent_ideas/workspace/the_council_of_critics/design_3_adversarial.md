# Design 3: The Adversarial Gym (Red Team)

## Purpose
A "Builder vs. Breaker" agent that gamifies quality assurance. Instead of polite feedback, this agent actively tries to break your code by generating hostile unit tests, fuzzing inputs, or finding security exploits. It maintains a "Scoreboard" of how many times it has humiliated the user.

## Loop Structure
1. **Commit Hook**: Triggered on `git commit`.
2. **Analysis**: "The Breaker" scans the diff for new surface area (new APIs, new inputs).
3. **Attack Generation**:
    *   Uses LLM to brainstorm edge cases (SQL injection, overflow, race conditions).
    *   Generates a Python/Shell script or a new unit test file (e.g., `tests/exploit_20251125.py`).
4. **Execution**:
    *   Runs the exploit via `shell`.
    *   If the exploit *fails* (code is robust), User gets +1 point.
    *   If the exploit *succeeds* (code breaks), Agent gets +1 point and the commit is blocked.
5. **Trophy Room**: Successful exploits are saved in `trophies/` and must be "defeated" (made to fail) before the code can merge.

## Tool Usage
*   **shell**: Execute tests, run build, run fuzzers.
*   **filesystem**: Write test files, read source.
*   **web**: Search for known CVEs or exploit patterns for the libraries being used.

## Memory Architecture
*   **Trophy Case**: A directory of historical exploits that serve as a regression suite.
*   **Scoreboard**: Simple KV store or file tracking wins/losses.

## Failure Modes
*   **Infinite Loop**: Agent keeps generating exploits that are theoretically valid but practically impossible (e.g., cosmic ray bit flips).
*   **Resource Exhaustion**: Fuzzing takes too long.

## Human Touchpoints
*   User is in a "combat" relationship with the agent.
*   User can "Challenge" the agent to find a bug in a specific file.
