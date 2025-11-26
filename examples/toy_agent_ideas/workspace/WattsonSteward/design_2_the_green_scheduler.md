# Design 2: The Green Scheduler (Innovative)

## Purpose
An active "GreenOps" daemon that intercepts and delays heavy, non-blocking compute tasks (builds, model training, backups) until the local electricity grid reaches a renewable energy peak.

## Loop Structure
1. **Forecast:** Polls the Carbon Aware SDK / ElectricityMaps API for the next 24h carbon intensity forecast.
2. **Queue:** Intercepts commands (via a wrapped shell alias or CI webhook) and places them in a "Green Queue".
3. **Execute:** Wakes up when grid intensity drops below a threshold (e.g., < 150gCO2/kWh) and executes the queued tasks.

## Tool Usage
- **web:** Queries Carbon Aware API.
- **shell:** Manages the task queue (using `at` or a custom scheduler), executes tasks.
- **memory:** Learns the "Deadline Tolerance" of users (e.g., "User X cancels tasks if delayed > 2h").
- **filesystem:** Logs execution times and carbon savings.

## Memory Architecture
- **Nodes:** `TaskType`, `UserTolerance`, `GridForecast`.
- **Edges:** `TaskType --HAS_TOLERANCE--> Duration`.
- **Logic:** "Optimize StartTime such that (CarbonIntensity * Energy) is minimized, subject to StartTime < Now + Tolerance."

## Failure Modes
- **Missed Deadlines:** If the grid is dirty for 24h, the agent must eventually run the task anyway.
- **Conflict:** User manually runs a task while agent is holding it. Agent detects this and dequeues.

## Human Touchpoints
- **Opt-in:** Users alias their commands (e.g., `gmake` instead of `make`).
- **Override:** `--now` flag forces immediate dirty execution.
