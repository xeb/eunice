# Agent: The Interface Cartographer

## 1. Vision
**"The Map is not the Territory — until verified."**
The Interface Cartographer is an autonomous agent dedicated to exploring, verifying, and mapping the behavior of CLI tools and API endpoints. Unlike static documentation which goes stale, the Cartographer builds a **Verified Behavior Graph** by actively running commands in a sandbox and recording the actual outcomes.

It answers the question: *"If I run 'X', what EXACTLY happens?"*

## 2. Core Architecture

### Toolset
*   **shell_execute_command:** The "Hands". Used to run the target tools in controlled experiments.
*   **memory (Graph):** The "Brain". Stores the verified ontology of Tools, Commands, Flags, and Outputs.
*   **web_brave_web_search:** The "Eyes". Reads official documentation to form initial hypotheses.
*   **grep (Text Processing):** The "Ears". Parses stdout/stderr to extract schemas and error codes.

### The Feedback Loop (The "Scientific Method")
1.  **Hypothesis (Read):** Agent searches web docs for a tool (e.g., "ffmpeg"). It learns that `-i` means input.
2.  **Experiment Design (Plan):** Agent formulates a safe test command: `ffmpeg -i input.mp4`.
3.  **Sandbox Execution (Act):** Agent executes the command (or a dry-run equivalent) via `shell`.
4.  **Observation (Measure):** Agent captures exit code, stdout, and stderr.
    *   *Observation:* "Exit Code 1. Stderr says 'Missing output file'."
5.  **Conclusion (Record):** Agent updates Memory:
    *   *Fact:* `ffmpeg -i [file]` is incomplete.
    *   *Constraint:* Requires output argument.
6.  **Iteration (Explore):** Agent tries `ffmpeg -i input.mp4 output.avi`. Success.

## 3. Persistence Strategy: The Behavior Graph
Instead of just writing text files, the agent builds a queryable knowledge graph:

*   **Entities:** `Tool`, `Command`, `Flag`, `OutputSchema`, `ErrorPattern`.
*   **Relations:**
    *   `Command` *REQUIRES* `Flag`
    *   `Command` *PRODUCES* `OutputSchema`
    *   `ErrorPattern` *INDICATES* `MissingDependency`

**Example Graph Node:**
```json
{
  "entityType": "Command",
  "name": "git status",
  "observations": [
    "Safe to run repeatedly (Idempotent)",
    "Returns Exit Code 0 on success",
    "Output format: 'On branch [BranchName]'"
  ]
}
```

## 4. Safety & Autonomy

### Autonomy Level: Semi-Autonomous Explorer
*   **Bounded Exploration:** The agent is given a "Sandbox" (a specific directory or Docker container). It is free to run *any* command within that sandbox.
*   **Destructive Guardrails:** The agent uses a heuristic (and web search) to identify "destructive" verbs (rm, delete, drop, prune) and requires Human Approval before running them, or runs them only on dummy files it created itself.

### Failure Recovery
*   **Infinite Loops:** Shell commands have strict timeouts (e.g., 5s).
*   **System Crash:** The agent logs the "Crasher Command" to Memory as a "Danger Node" and avoids similar patterns.

## 5. Use Cases
1.  **Agent Training Ground:** Other AI agents query the Cartographer's graph to learn how to use tools safely ("How do I list files in S3 without deleting them?").
2.  **Documentation Regression Testing:** Run the Cartographer against a new version of a CLI to see if flags changed behavior.
3.  **Legacy Code Analysis:** "We have this binary from 2015. What does flag `-x` do?" The Cartographer finds out by trying it.

## 6. Novelty
Most agents *use* tools to do a task. The Interface Cartographer *studies* tools to understand them. It is an **epistemological agent**—it builds knowledge about the digital world itself.
