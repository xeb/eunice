# Agent Design: The Process Symbiont

## Executive Summary
The Process Symbiont is an autonomous background daemon that ensures the reliability of local development environments. Unlike traditional process managers (which just restart) or CI agents (which run in isolation), the Symbiont "lives" in the developer's active workspace. It monitors running services, diagnoses crashes using web search, applies learned fixes (e.g., clearing lock files, killing port zombies), and maintains a persistent "Health Graph" of the system.

## Core Toolset
*   **Shell**: For process discovery (`ps`, `lsof`), log monitoring (`tail`), and execution of fix commands.
*   **Memory**: To build a knowledge graph of `Services`, `Dependencies`, `ErrorPatterns`, and `ProvenFixes`.
*   **Web (Brave)**: To research obscure error codes and find solutions for new crashes.
*   **Grep**: To analyze local configuration files (`docker-compose.yml`, `package.json`) to infer intended state.

## Architecture

### 1. The Observation Loop (Daemon)
*   **Discovery**: Every 10s, scans the process table to identify "Dev Processes" (node, python, postgres, docker).
*   **Health Check**: Monitors:
    *   **Liveness**: Is the PID valid?
    *   **Port Availability**: Is `localhost:3000` responding?
    *   **Log Sentiment**: Tail stderr for keywords like "Error", "Exception", "Fatal".

### 2. The Diagnosis Engine
When a failure is detected (Crash or Error Log):
1.  **Context Capture**: Grab last 50 lines of logs + environment vars.
2.  **Memory Lookup**: Query the Graph: "Have we seen this Error Pattern for this Service before?"
    *   *Match*: Retrieve the associated `ProvenFix`.
    *   *No Match*: Trigger **Research Mode**.
3.  **Research Mode**:
    *   Search Web for "Error message" + "Stack".
    *   Summarize top results into a "Candidate Fix" (e.g., "Delete node_modules", "Kill process on port 5432").

### 3. The Remediation System (The Immune Response)
*   **Safe Fixes**: (Restart, Kill Zombie, Clear Temp) -> **Autonomous Execution**.
*   **Risky Fixes**: (Delete Data, Reinstall, Modify Config) -> **Request Permission** (via Terminal Notification or `APPROVAL_NEEDED.md`).
*   **Feedback Loop**: If a fix works (service stays up for >10 mins), store it in Memory as a `ProvenFix`.

## Memory Graph Schema
*   **Nodes**:
    *   `Service`: { name: "backend", cmd: "npm start", port: 3000 }
    *   `ErrorPattern`: { signature: "EADDRINUSE", regex: "Address already in use" }
    *   `Fix`: { cmd: "kill -9 $(lsof -t -i:3000)", type: "safe" }
*   **Edges**:
    *   `Service` --HAS_EXPERIENCED--> `ErrorPattern`
    *   `ErrorPattern` --SOLVED_BY--> `Fix`
    *   `Fix` --APPLIES_TO--> `Service`

## Example Workflow: The "Zombie Port" Scenario
1.  **Situation**: User Ctrl-C's the backend, but the node process hangs, holding port 3000.
2.  **User Action**: User tries to run `npm start` again.
3.  **Failure**: New process crashes with `EADDRINUSE: 3000`.
4.  **Symbiont Action**:
    *   Detects crash & error log.
    *   Recognizes `EADDRINUSE`.
    *   Checks Memory -> Knows `Fix: Kill Port Owner` is safe and effective.
    *   Executes: `lsof -t -i:3000 | xargs kill -9`.
    *   Restarts the user's command.
5.  **Result**: The server starts successfully without the user needing to manually debug.

## Failure Modes & Safety
*   **The "War of the Daemons"**: If the Symbiont fights with another manager (e.g., Docker restart policy). *Mitigation*: Check parent PIDs.
*   **Destructive Hallucination**: AI suggests `rm -rf /`. *Mitigation*: Strict regex allowing only specific commands (npm, kill, rm specific extensions).

## Practical Value
Reduces "Environment Friction"—the 10-20% of dev time lost to restarting flaky services, clearing caches, and debugging non-code environment issues.
