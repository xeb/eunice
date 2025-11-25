# Design 3: The Symbiotic Sidecar (Collaborative)

## Purpose
To act as a "Co-Pilot for Operations," maintaining a shared mental model of the development environment. Instead of silently fixing things, it *orchestrates* the workflow and provides context-aware suggestions in real-time.

## Problem
Developers often forget the complex incantations needed to start a project (e.g., "Run db, then redis, then backend with FLAG=1"). Documentation is often out of date.

## Loop Structure
1.  **Observation**: Watch file changes (filesystem) and running processes (shell).
2.  **Context Building**: Match current activity to "Project Runbooks" stored in Memory.
3.  **Suggestion Engine**:
    *   *User edits `prisma.schema`*: Agent suggests "Do you want to run `prisma migrate`?"
    *   *User starts backend but Redis is off*: Agent prompts "Redis is required but not running. Start it?"
4.  **Interface**: A "Sidecar File" (e.g., `_AGENT_SUGGESTIONS.md`) or terminal notifications.
5.  **Learning**: If user runs a command sequence frequently, the Agent proposes adding it to the Runbook.

## Tool Usage
*   **memory**: Stores the "Project Graph" (Dependencies: Backend -> Redis).
*   **filesystem**: Watches for file modification events (triggers).
*   **grep**: Scans `package.json`, `Makefile`, `Dockerfile` to infer relationships.

## Memory Architecture
*   **Entities**: `Task` (e.g., "Run Migrations"), `Trigger` (File Change), `Dependency`.
*   **Graph**: Encodes the *logic* of the dev environment (Topology).

## Failure Modes
*   **Annoyance**: Too many suggestions ("Clippy" problem).
*   **Stale Context**: Suggesting commands that are no longer relevant.

## Human Touchpoints
*   **High Interaction**: The agent is constantly proposing; the human is deciding.
*   **Runbook Approval**: Human verifies learned patterns.

## Autonomy Level
**Collaborative**. The agent proposes actions based on deep context; the user acts.
