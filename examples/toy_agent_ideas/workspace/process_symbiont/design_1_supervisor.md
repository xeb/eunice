# Design 1: The Smart Supervisor (Conservative)

## Purpose
To provide robust, reliable management of local development processes (servers, databases, compilers) with AI-enhanced error categorization, ensuring high uptime without risky autonomous interventions.

## Problem
Developers often juggle multiple terminal tabs running services (backend, frontend, db). When one crashes, it disrupts flow. Standard tools like `pm2` or `restart: always` are "dumb"—they restart even if the error is fatal (loops) or don't restart if the error looks clean but isn't.

## Loop Structure
1.  **Monitor Loop**: Check PIDs and HTTP health endpoints every 5 seconds.
2.  **Crash Detection**: If a process exits, capture the last 50 lines of logs.
3.  **Diagnosis (AI)**: Send logs to LLM. Classify as:
    *   *Transient* (Network blip, timeout) -> RESTART.
    *   *Fatal* (Syntax error, missing config) -> STOP & NOTIFY.
    *   *Resource* (OOM) -> RESTART with Warning.
4.  **Action**: Execute the decision.
5.  **Log**: Record the incident in `process_health.log`.

## Tool Usage
*   **shell**: To spawn processes, check `ps`, tail logs.
*   **memory**: To store "Stability Scores" for each service. If a service crashes 3x in 10 mins, downgrade its score and stop restarting.
*   **filesystem**: Read `supervisor.config.json` (user defined) and write logs.

## Memory Architecture
*   **Entities**: `Service` (name, start_cmd), `Incident` (timestamp, error_log, classification).
*   **Relations**: `Service` HAS_MANY `Incidents`.
*   **Usage**: "Has this service crashed with this specific error before?" If yes, and we ignored it, maybe escalate now.

## Failure Modes
*   **False Positive**: AI thinks a syntax error is transient (unlikely given good prompting).
*   **Restart Loop**: Controlled by the "Stability Score" in memory.
*   **Resource Hog**: `tail` on a massive log file could slow things down.

## Human Touchpoints
*   **Configuration**: User must explicitly list commands in `supervisor.config.json`.
*   **Notification**: When a process is permanently stopped, the agent leaves a `CRASH_REPORT.md` in the root.

## Autonomy Level
**Low/Safe**. It only restarts; it never modifies code or changes environment configurations.
