# Design 2: The Process Operator

## Purpose
An active agent that acts as a local DevOps engineer. It starts, stops, and restarts services based on a declarative "Goal State".

## Loop Structure
1. **Read Goal:** Check `local-infra.yaml` (user defined) for required services.
2. **Scan:** Check current running processes.
3. **Diff:** Identify missing services or zombies (processes on ports that shouldn't be there).
4. **Act:**
   - **Start:** Run `npm start` or `docker-compose up` for missing services.
   - **Kill:** Terminate zombie processes blocking required ports.
5. **Verify:** Check if the service came up healthy (curl localhost:PORT).

## Tool Usage
- **shell:** `kill`, `npm`, `docker`, `curl`.
- **filesystem:** Read logs to debug startup failures.
- **memory:** Track "Backoff" state (don't keep restarting a crashing service).

## Memory Architecture
- **State Machine:** Tracks `STOPPED`, `STARTING`, `RUNNING`, `CRASH_LOOP`.
- **Health History:** "Service X fails 50% of the time on Monday mornings."

## Failure Modes
- **Destructive Kill:** Accidentally killing a system process (needs whitelist).
- **Infinite Loop:** Trying to start a broken service forever.

## Human Touchpoints
- **Confirmation:** Ask before killing a process not owned by the agent.
- **Config:** User defines the `local-infra.yaml`.

