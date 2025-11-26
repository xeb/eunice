# Design 3: The Invariant Hunter (Hybrid)

## Purpose
A "Chaos Engineering" agent that doesn't just break things, but actively verifies "Soft Invariants" (rules that should always be true). It runs alongside integration tests or manual usage, observing the system via `grep` on logs and `shell` queries to databases. It learns "Normal" behavior and flags deviations.

## Core Loop
1. **Learn**: Observes successful test runs to build a baseline of "Invariants" (e.g., "Response time < 500ms", "User balance never negative", "Order ID is monotonic"). Stores these in `memory`.
2. **Disturb**: Uses `shell` to inject faults (kill process, limit network, fill disk) or generate edge-case data.
3. **Monitor**: Greps logs and queries state to see if Invariants hold.
4. **Report**: If an Invariant breaks, it records the exact "Disturbance Vector" and the "Broken Invariant" as a "Vulnerability Candidate".
5. **Verify**: Attempts to reproduce the break to confirm it wasn't a fluke.

## Tool Usage
- **shell**: To inject faults (`kill`, `iptables`, `dd`) and run queries.
- **grep**: Real-time log monitoring (tail -f | grep).
- **memory**: Storing the library of Invariants and their health status.
- **filesystem**: Writing "Anti-Regression Tests" (tests that prove the bug).

## Memory Architecture
- **Entities**: `Invariant` (logic: "x > 0"), `Disturbance` (type: "network_latency"), `Component`.
- **Relations**: `holds_for`, `broken_by`, `monitors`.

## Failure Modes
- **False Positives**: Agent flags normal spikes as bugs. *Mitigation:* Statistical thresholds (Sigma rules).
- **Over-Aggressive**: Agent kills the database during a demo. *Mitigation:* "Safe Mode" flag, time windows (only run at night).

## Human Touchpoints
- **Invariant Approval**: Humans review the "Inferred Invariants" to confirm they are actually rules.
- **Trophy Case**: Agent lists "Bugs Found" in a README.
