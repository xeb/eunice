# Design 2: The Black Box Explorer

## Purpose
To autonomously discover the capabilities of a software tool *without* relying on documentation. This approach is useful for legacy binaries, undocumented APIs, or malware analysis.

## Core Philosophy
"Poke it and see what happens." Use reinforcement learning and fuzzing strategies to map the "State Space" of the CLI tool.

## Loop Structure
1.  **Discovery:** Run `--help` or `strings` on the binary to find potential flags and keywords.
2.  **Hypothesis Generation:** The LLM guesses valid command structures (e.g., `[binary] [verb] [flag]`).
3.  **Active Probing:**
    *   Execute commands in a highly isolated sandbox.
    *   Monitor exit codes, output length, and execution time.
4.  **Signal Analysis:**
    *   If Exit Code 0: "Valid Command".
    *   If Exit Code 1 + "Unknown flag": "Invalid Syntax".
    *   If Exit Code 1 + "Missing argument": "Partial Syntax" (Progress!).
5.  **Refinement:** Use the feedback to generate more complex commands.
6.  **Mapping:** Construct a dependency graph (e.g., "Command B only works after Command A sets up a config").

## Tool Usage
*   **shell_execute_command:** The primary probe. High volume of calls.
*   **grep_count_matches:** To analyze log files or huge outputs quickly.
*   **memory_add_observations:** To record the "Physics" of the tool (e.g., "Arg 1 must be an integer").
*   **filesystem_read_file:** To check if files were created or modified by the tool.

## Memory Architecture
*   **Graph:** A state machine. Nodes are "Tool States" (e.g., Uninitialized, Authenticated, Error). Edges are "Commands".
*   **Entity:** `SyntaxPattern` (e.g., "--user <string>")
*   **Observation:** "Pattern X triggers a Segfault."

## Failure Modes
*   **System Hangs:** The tool enters an infinite loop.
    *   *Mitigation:* Strict timeouts on `shell_execute_command`.
*   **Resource Exhaustion:** The tool creates 1GB log files.
    *   *Mitigation:* Monitor disk usage and kill process if limits exceeded.
*   **Side Effects:** The tool deletes files.
    *   *Mitigation:* **Ephemeral Containers**. Every run happens in a throwaway environment (e.g., Docker).

## Human Touchpoints
*   **Boundary Setting:** Human defines the "Blast Radius" (allowed directories, network access).
*   **Insight Extraction:** Human queries the agent: "Did you find any hidden debug flags?"
