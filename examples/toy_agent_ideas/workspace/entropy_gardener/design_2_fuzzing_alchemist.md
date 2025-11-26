# Design 2: The Fuzzing Alchemist

## Purpose
The Fuzzing Alchemist is an offensive agent that generates "evil" inputs to find edge cases, crashes, and unhandled exceptions. Unlike standard fuzzers that use random bit-flipping, this agent uses LLM inference and "Property Retrieval" to generate semantically plausible but dangerous inputs (e.g., SQL injection strings, huge JSON payloads, boundary dates) based on the function's type signature and documentation.

## Loop Structure
1.  **Discovery:** Scan the codebase for public API endpoints or core data processing functions.
2.  **Property Retrieval:** Use `grep` and `web_search` (if docs are online) to understand the *intended* invariants (e.g., "age must be positive", "username must be unique").
3.  **Synthesis:** Generate a "Fuzz Suite" of inputs:
    *   *Type Boundary:* MaxInt, MinInt, Null, Empty String.
    *   *Semantic:* Emojis in ASCII fields, SQL-like strings, recursive JSON.
4.  **Attack:** Execute the functions with these inputs in a sandboxed process.
5.  **Observation:** Monitor for:
    *   Uncaught Exceptions / Crashes
    *   Timeouts
    *   Memory spikes
6.  **Reporting:** If a crash is found, minimize the input (find the smallest string that still crashes it) and create a reproduction script.

## Tool Usage
*   **text-editor:** Read function signatures and docstrings.
*   **shell:** Run Python/Node scripts with generated inputs; monitor process exit codes.
*   **memory:** Store "Effective Fuzz Patterns" (inputs that triggered bugs in the past) to reuse across projects.
*   **filesystem:** Save crash logs and "Reproduction Recipes".

## Memory Architecture
*   **Nodes:** `Function`, `InputPattern`, `Crash`
*   **Relations:**
    *   `(InputPattern) CRASHED (Function)`
    *   `(Function) RESISTANT_TO (InputPattern)`
*   **Learning:** If "Recursive JSON" crashes one JSON parser, the agent boosts the priority of that pattern for *all* other parsing functions in the graph.

## Failure Modes
*   **Side Effects:** Fuzzing a "delete user" function might actually delete data. **Recovery:** STRICT SANDBOXING. The agent should only fuzz in a Docker container or mock the database layer.
*   **False Alarms:** Crashing on invalid input might be intended behavior (if it throws a proper error). **Recovery:** The agent parses the exception type. `ValueError` is fine; `SegFault` or `500 Internal Error` is a bug.

## Human Touchpoints
*   **Dashboard:** A local HTML file showing "Crash Clusters".
*   **New Issue:** Can draft a GitHub issue with the reproduction steps.
