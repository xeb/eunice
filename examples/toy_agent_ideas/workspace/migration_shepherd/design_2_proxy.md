# Design 2: The Traffic Shaper (Innovative)

## Purpose
To migrate entire microservices or API endpoints by acting as an intelligent "Sidecar" agent that manages traffic routing and verification transparently.

## Loop Structure
1. **Intercept:** Agent starts a local proxy server (e.g., on port 3000) using `shell` (node/python script).
2. **Forward:** It forwards incoming requests to the `Legacy Service` (port 3001).
3. **Shadow:** It asynchronously forwards the same request to the `Candidate Service` (port 3002).
4. **Compare:** It captures both responses (status, body, headers).
5. **Analyze:** It computes a "Semantic Diff" (ignoring timestamps/IDs) and stores it in **Memory**.
6. **Learn:** It clusters mismatches. (e.g., "Candidate service always fails on users with ID > 1000").
7. **Promote:** When error rate is 0% for a cluster, it updates the routing rule to send that traffic to Candidate primarily.

## Tool Usage
- **shell:** Spawning the proxy and the services.
- **fetch:** (Used internally by the proxy, or for active probing).
- **memory:** Storing the "Request Signature" -> "Match Status" graph.
- **web:** Searching for known breaking changes in dependencies if mismatches occur.

## Memory Architecture
- **Nodes:** `RequestPattern`, `DivergenceEvent`.
- **Relations:** `CAUSES_DIVERGENCE`, `IS_SAFE_FOR`.
- **Persistence:** Memory Graph allows the agent to answer questions like "Which endpoints are safe to switch right now?"

## Failure Modes
- **Latency:** Double-processing adds delay (though shadow is async).
- **State Drift:** If services share a DB, shadow writes must be suppressed (Mocking).
- **Recovery:** Agent auto-kills Candidate service if it crashes the Proxy.

## Human Touchpoints
- User configures the "Shadow Mode" (Read-only vs Mock-Write).
- User confirms the final teardown of Legacy Service.
