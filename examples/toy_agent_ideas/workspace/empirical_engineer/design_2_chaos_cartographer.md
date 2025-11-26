# Design 2: The Chaos Cartographer

**Theme:** Active, experimental interrogation of the system.

## Purpose
To understand system boundaries and reproduce "heisenbugs" by actively manipulating the environment. Unlike the Falsificationist, this agent believes "you don't know it until you can break it."

## Core Loop: The Perturbation Cycle
1.  **Baseline:** Measure current system state (metrics, logs).
2.  **Causal Inference:** Build a graph of likely dependencies in `memory`.
3.  **Active Experiment:** Design a perturbation to test a link in the graph.
    *   *Example:* "If I block port 8080, does the service degrade gracefully or crash?"
    *   *Example:* "If I fill the disk to 99%, does the logger handle it?"
4.  **Execution:** Apply the stressor/change via `shell` or `filesystem`.
5.  **Observation:** Record the delta between Baseline and Perturbed state.
6.  **Mapping:** Update the `memory` graph with confirmed causal links and failure modes.

## Tool Usage
*   **shell:** Full access. Used to stop services, firewall ports, generate load, modify env vars.
*   **filesystem:** modify config files, create large temp files.
*   **memory:** Stores a "Causal Graph" of the system. Nodes are Components, Edges are "Influences".

## Memory Architecture
*   **Entities:** `Component`, `Stressor`, `Reaction`.
*   **Relations:**
    *   `Stressor CAUSES Reaction`
    *   `Component VULNERABLE_TO Stressor`

## Failure Modes
*   **Destruction:** Accidentally causing permanent data loss or system instability.
*   **Heisenberg Effect:** The instrumentation/stressor changes the behavior so much that the original bug is masked.

## Human Touchpoints
*   **Permission:** Requires explicit approval for "High Risk" categories of experiments (e.g., stopping DB).
*   **Cleanup:** Human might need to intervene if the agent fails to revert state.
