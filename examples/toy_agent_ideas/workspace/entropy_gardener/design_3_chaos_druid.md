# Design 3: The Chaos Druid

## Purpose
The Chaos Druid does not test code; it tests *resilience*. It assumes the code works, but the world does not. It acts as a background daemon that randomly sabotages the local development environment—deleting `node_modules`, creating read-only files, disconnecting the network, or killing database processes—to ensure the application handles environmental failure gracefully.

## Loop Structure
1.  **Mapping:** Identify dependencies (databases, external APIs, file paths) using `lsof` or analyzing config files.
2.  **Plan Sabotage:** Select a "Disaster Scenario":
    *   *The Vanishing:* Rename a required config file.
    *   *The Slowdown:* Use `tc` (traffic control) to add 2000ms latency to localhost.
    *   *The Zombie:* Pause the database process (`kill -STOP`).
    *   *The Lock:* `chmod 000` a log file.
3.  **Execute:** Apply the sabotage for a fixed duration (e.g., 30 seconds).
4.  **Monitor:** Watch application logs. Does it crash? Does it retry? Does it log a helpful error?
5.  **Restore:** Revert the sabotage (rename file back, `kill -CONT`).
6.  **Report:** specific "Resilience Score" based on recoverability.

## Tool Usage
*   **shell:** The primary weapon. `mv`, `chmod`, `kill`, `iptables` (if root), `docker`.
*   **filesystem:** Identify critical assets to target.
*   **grep:** Scan logs for "Connection Refused" vs "Unhandled Exception".
*   **memory:** specific scenarios that caused catastrophic failure.

## Memory Architecture
*   **Nodes:** `Service`, `FailureMode`, `Outcome`
*   **Relations:**
    *   `(Service) SURVIVED (FailureMode)`
    *   `(Service) CRASHED_BY (FailureMode)`
*   **Persistence:** Learns which services are "fragile" and focuses testing there.

## Failure Modes
*   **Destructive Irreversibility:** Deleting a file that cannot be restored. **Recovery:** Only use *renaming* or *permissions*, never delete. or use a git-clean strategy.
*   **System Instability:** Crashing the developer's entire machine. **Recovery:** Limit scope to specific PIDs or Docker containers; never touch system processes.

## Human Touchpoints
*   **"Game Day" Mode:** The agent only runs when explicitly toggled ON, perhaps during a dedicated "Resilience Testing" hour. It should not run while the dev is trying to ship a hotfix.
