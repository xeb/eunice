# Design 2: The State Cartographer (Innovative)

## Purpose
An agent that autonomously maps the "state space" of a running application. Instead of random fuzzing, it attempts to infer a State Machine model of the software (e.g., "Login -> Dashboard -> Settings") and deliberately searches for edges (transitions) that are unmapped or throw errors. It is a "Black Box" explorer that builds a "White Box" model.

## Core Loop
1. **Hypothesize**: Based on `memory` and static analysis (`grep`), the agent predicts a possible state transition (e.g., "If I POST to /api/reset, state should go to Init").
2. **Experiment**: Generates a script (curl, python, puppeteer) to attempt this transition.
3. **Observe**: Captures side effects (logs, exit codes, file changes) and the new state.
4. **Update Map**: 
   - If successful, creates a `Transition` relation in `memory`.
   - If error, creates an `Anomaly` entity.
   - If the state is new, creates a `State` entity.
5. **Strategize**: Uses graph traversal algorithms (BFS/DFS) on the Memory Graph to find the "frontier" of unexplored states.

## Tool Usage
- **memory**: The primary storage. Stores the graph of States (Nodes) and Actions (Edges).
- **shell**: To execute the interaction scripts.
- **grep**: To scan source code for "clues" about hidden states (e.g., finding `enum State { IDLE, BUSY }` or API route definitions).
- **web**: To search for documentation on standard protocols (e.g., "What is the standard error code for X?").

## Memory Architecture
- **Entities**: `State` (attributes: url, memory_usage, prompt), `Action` (input vector), `Invariant`.
- **Relations**: `transitions_to` (with probability), `blocked_by`, `violates`.
- **Inference**: "If Action A always leads to State B, but today led to State C, flag as Regression."

## Failure Modes
- **State Explosion**: The map becomes too large. *Mitigation:* Cluster states by similarity (LSH) and treat them as one node.
- **Destructive Actions**: Agent deletes production data. *Mitigation:* Agent only runs in a sandboxed environment (Docker container provided by user).

## Human Touchpoints
- **Map Visualization**: Agent generates a Graphviz/Mermaid file of the discovered state machine for the human to audit.
- **Guidance**: Human can "pin" certain states as critical to explore.
