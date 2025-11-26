# Design 2: The Autonomous Healer

## Purpose
To actively monitor system logs and execute known remediation strategies ("Heals") without human intervention, effectively functioning as a digital SRE.

## Loop Structure
1. **Log Tailing**: Continuously runs `tail -f /var/log/syslog` (or other target logs) via `shell`.
2. **Pattern Matching**: Matches log lines against signatures stored in `memory`.
3. **Diagnosis**: If a match is found (e.g., "Out of Memory"), it looks up the linked `Remediation` subgraph.
4. **Execution**: It executes the `Remediation` commands (e.g., `docker restart web_app`) using `shell`.
5. **Verification**: Checks if the log error stops or if a "Success" metric is met.
6. **Learning**: Updates the `ConfidenceScore` of that remediation in `memory`.

## Tool Usage
*   **shell**: Executing fixes, restarting services, tailing logs.
*   **memory**: Storing `Symptom -> Diagnosis -> Fix` mappings and success rates.
*   **web**: (Optional) Searching StackOverflow if a new error code is encountered (for "Drafting" new fixes).

## Memory Architecture
*   **Nodes**: `Symptom`, `Fix`, `Incident`.
*   **Relations**: `Symptom -> solved_by -> Fix`, `Fix -> has_risk_level -> Low/High`.
*   **Logic**: "If Symptom A appears, try Fix X (Confidence 90%). If that fails, try Fix Y (Confidence 40%)."

## Failure Modes
*   **Looping**: The fix causes the error to happen again immediately (CrashLoopBackOff). **Mitigation**: Max retry count per hour.
*   **Destructive Acts**: A bad fix could delete data. **Mitigation**: Only allow "Read-Only" or "Safe-Listed" commands (e.g., restarts) without human approval.

## Human Touchpoints
*   **Escalation**: If no fix works, it pings the human (via a file `ALERTS/HELP_ME.md` or notification).
*   **Training**: Human must "Authorize" new complex fixes before they become fully autonomous.
