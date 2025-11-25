# Design 2: The Autonomic Admin (Innovative)

## Purpose
To autonomously heal the local development environment by actively diagnosing errors, searching for solutions, and executing system-level fixes (killing zombies, clearing locks, installing missing deps).

## Problem
"Restarting" isn't enough when the error is `EADDRINUSE: port 3000 already in use` or `Module 'lodash' not found`. These require active intervention. Developers waste hours searching for obscure error codes.

## Loop Structure
1.  **Discovery**: Scan open terminals (via `ps` or tmux/screen integration) to find running dev processes without explicit config.
2.  **Health Check**: Periodically curl localhost ports or grep log tails for "Error"/"Exception".
3.  **Investigation**:
    *   If error detected, **Search Web** (Brave) for the error message + context.
    *   **Search Memory** for past successful fixes.
4.  **Remediation Plan**:
    *   *Port Conflict*: `lsof -i :3000` -> `kill -9 <pid>`.
    *   *Missing Dep*: `npm install <package>`.
    *   *Lock File*: `rm index.lock`.
5.  **Execution**: Run the fix command.
6.  **Verification**: Restart service and verify health.

## Tool Usage
*   **shell**: Aggressive use: `lsof`, `kill`, `npm`, `docker`.
*   **web**: Search stackoverflow/github issues for error strings.
*   **grep**: Scan codebase for config files to understand what *should* be running.
*   **memory**: "Knowledge Base of Fixes". Entity: `ErrorPattern`, Relation: `SOLVED_BY` -> `Command`.

## Memory Architecture
*   **Self-Improving**: It remembers that "Error X" requires "Fix Y".
*   **Context**: Maps specific repos to their specific quirks (e.g., "Repo A needs Node 14").

## Failure Modes
*   **Destructive Fixes**: Killing the wrong process or deleting a needed file.
*   **Hallucination**: Executing a dangerous command found on the web (needs strict filtering/sandboxing).

## Human Touchpoints
*   **Permission**: Default to "Ask for Permission" for destructive commands (delete/kill), but "Autonomous" for additive ones (install).

## Autonomy Level
**High**. Can modify system state (processes, files) based on external data (web search).
