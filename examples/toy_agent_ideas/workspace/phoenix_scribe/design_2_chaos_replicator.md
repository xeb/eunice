# Design 2: The Chaos Replicator (Innovative)

## Purpose
To move beyond code-level bugs and address *environmental* failures (network, disk, memory) described in Post-Mortems by orchestrating local chaos simulations.

## Loop Structure
1. **Ingest**: Read PMs and classify the failure type (Code Logic vs. Environmental).
2. **Orchestrate**: If Environmental (e.g., "DB Latency caused timeout"), generate a `chaos_config.yaml` (for tools like Pumba, Toxiproxy, or Gremlin).
3. **Simulate**: Spin up the application in a containerized environment (using `shell`).
4. **Attack**: Apply the chaos defined in the config (e.g., add 500ms delay to DB port).
5. **Observe**: Monitor logs and health endpoints.
6. **Assert**: Verify if the system degrades gracefully (as per the "Fix" described in PM) or crashes.

## Tool Usage
- **shell**: Heavy usage. `docker-compose up`, `tc` (traffic control), `kill -9`.
- **web_brave**: Research specific chaos engineering patterns for detected technologies (e.g., "how to simulate redis packet loss").
- **filesystem**: Read logs to detect success/failure.

## Memory Architecture
- **Experiment Log**: specific folder `experiments/` tracking which chaos scenarios have been run and their outcomes.

## Failure Modes
- **Destructive**: Could accidentally kill the agent's own process or system-critical processes if not sandboxed. *Mitigation:* strict strict limits on PID ranges or container-only scope.
- **False Positives**: The chaos might be too strong (100% packet loss), causing failure regardless of code quality.

## Human Touchpoints
- **Permission**: Explicit approval required before running any "Active" attack, even locally.
- **Tuning**: Humans provide feedback on whether the simulation accurately reflects the incident.
