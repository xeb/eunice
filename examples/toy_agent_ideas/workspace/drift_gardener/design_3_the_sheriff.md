# Design 3: The Sheriff (Policy Enforcement)

## Purpose
"The Sheriff" is an autonomous governance agent. Instead of just mirroring or fixing code, it actively enforces a high-level `policy.md` or Open Policy Agent (OPA) ruleset. It is designed to reduce costs and security surface area by aggressively terminating "Zombie" or "Non-Compliant" resources that violate the rules.

## Loop Structure
1.  **Policy Loading:** Reads `policy.md` (e.g., "All EC2 instances must have a 'Department' tag", "No S3 buckets publicly readable", "Dev resources TTL = 24h").
2.  **Audit:** Scans cloud infrastructure and builds a list of violations.
3.  **The Trial:** For each violation, it checks the Memory Graph:
    *   *First Offense:* Sends a warning notification (writes to `alerts.log`). Tags the resource `TerminationScheduled: <Date>`.
    *   *Repeat Offense:* Checks if the `TerminationScheduled` time has passed.
4.  **Execution:** If time is up, it executes the termination command (`aws ec2 terminate-instances ...`).
5.  **Record Keeping:** Logs the "Death Certificate" of the resource in the Memory Graph and an `audit_log.csv`.

## Tool Usage
*   **Shell:** `aws`, `gcloud`, `opa` (Open Policy Agent).
*   **Filesystem:** Read `policy.md`, write logs.
*   **Memory:** Maintain the "Criminal Record" of resources (Warning counts, Grace periods).
*   **Web:** (Optional) Search for vulnerability CVEs to prioritize termination.

## Memory Architecture
*   **Nodes:** `Violation`, `Resource`, `Policy`.
*   **Edges:** `Resource` -> `Violation` (COMMITTED), `Violation` -> `Policy` (VIOLATES).
*   **Properties:** `grace_period_end`, `owner_contacted`.

## Failure Modes
*   **Collateral Damage:** Deleting a critical production resource because it lacked a tag.
    *   *Mitigation:* "Doomsday Switch" - Agent requires a specialized `allow_terminate: true` flag in config, and enforces a "Max Kill Count" per run (e.g., max 5 deletions).
*   **Policy Ambiguity:** LLM misinterprets a natural language policy.
    *   *Mitigation:* Use OPA/Rego for strict logic; LLM only translates `policy.md` to Rego.

## Human Touchpoints
*   **The Pardon:** Human can add a `DoNotDelete` tag to a resource to grant it immunity.
*   **Policy Authoring:** Human writes the laws; Agent enforces them.
