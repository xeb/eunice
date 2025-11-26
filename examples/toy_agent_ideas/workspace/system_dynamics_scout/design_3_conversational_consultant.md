# Design 3: The Resilience Consultant (Conversational)

## Purpose
A conversational partner that helps developers 'think in systems'. It doesn't parse text automatically; instead, it interviews the developer during the design phase to elicit the causal structure.

## Tool Usage
*   **memory**: Stores the evolving 'Mental Model' of the user.
*   **web**: Searches for 'Archetypes' (e.g., 'Tragedy of the Commons').
*   **shell**: Can query system metrics (Prometheus/Grafana) to verify claims.

## Loop Structure
1.  **Interview**: Agent asks: 'What is the goal of this new feature?'
2.  **Probing**: Agent asks: 'What variables influence latency?'
3.  **Loop Closing**: Agent asks: 'Does low latency affect cache hit rate?'
4.  **Archetype Matching**: Agent searches Memory for known patterns.
5.  **Metric Binding**: Agent binds PromQL query to the node.

## Human Touchpoints
*   High. Continuous dialogue.
