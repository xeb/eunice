# Design 1: The Passive Observer

## Purpose
A read-only agent that visualizes the current state of localhost. It answers "What is running where?" without modifying the system.

## Loop Structure
1. **Scan:** Run `lsof -i -P -n`, `docker ps`, and `ps aux` to list active processes and ports.
2. **Map:** Match PIDs to working directories (cwd).
3. **Identify:** Read `package.json` or `Makefile` in the cwd to guess the service name.
4. **Publish:** Generate a `status.md` or `dashboard.json` file in the user's home directory.
5. **Sleep:** Wait 60 seconds and repeat.

## Tool Usage
- **shell:** `lsof`, `ps`, `docker` (read-only commands).
- **filesystem:** Read config files to identify service names.
- **memory:** Store the snapshot to detect changes (drift).

## Memory Architecture
- **Transient:** Stores only the current snapshot.
- **History:** Simple log of "Service X started at [Time]".

## Failure Modes
- **Permission Denied:** Fails to read process info for root processes (graceful degradation).
- **Unknown Service:** Lists as "Unknown Process [PID]".

## Human Touchpoints
- **View:** User reads the generated dashboard.

