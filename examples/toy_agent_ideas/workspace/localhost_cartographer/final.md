# Agent: The Localhost Cartographer

## Purpose
A background agent that turns the opaque chaos of `localhost` into a queryable, semantic knowledge graph. It bridges the gap between **Runtime State** (processes, ports, logs) and **Source Truth** (code, config, docs), effectively acting as a "Live Documentation Generator" for your local development environment.

## Core Loop
1. **Topography Scan (Shell):**
   - Continuously monitors system state using `lsof`, `ps`, `docker ps`, and `netstat`.
   - Identifies every process listening on a TCP/UDP port.
   - Extracts Metadata: PID, Port, User, Start Time, Command Line Arguments.

2. **Provenance Tracing (Filesystem):**
   - Resolves the Working Directory (CWD) of each PID.
   - Locates project markers (`package.json`, `Cargo.toml`, `.git/`) to identify the "Project".
   - Parses configuration files (`.env`, `docker-compose.yml`) to understand intended relationships.

3. **Semantic Enrichment (Grep + Web):**
   - **Introspection:** Uses `grep` to find API routes (`@app.get`, `router.post`) within the source code of the running service.
   - **Identification:** Uses `web` search to identify unknown binaries or default ports (e.g., "What is listening on 5432?").
   - **Health Check:** Periodically pings detected endpoints (`/health`, `/`) to verify availability.

4. **Graph Synthesis (Memory):**
   - Updates the persistent Knowledge Graph with nodes: `Service`, `Port`, `Endpoint`, `Repo`, `Dependency`.
   - Links them: `Service(Frontend) --[TALKS_TO]--> Port(3000) --[OWNED_BY]--> Service(Backend)`.

5. **Interface Generation:**
   - **Passive:** Maintains a `workspace/status.md` (Live Dashboard) with a table of active services, health status, and quick links to code.
   - **Interactive:** Answers queries like "Why is my React app failing to connect to the API?" by traversing the graph (e.g., "The API is running on port 8000, but React is configured to hit 3000").

## Tool Usage
- **shell:** `lsof -i -P -n`, `ps aux`, `docker ps`, `curl`.
- **filesystem:** Reading config files, finding repos.
- **grep:** Scanning source code for routes and config usage.
- **memory:** Storing the "Runtime Graph" to track stability over time.
- **web:** Looking up error codes or default service ports.

## Key Insight: "The Runtime-Source Bridge"
Most tools look at *either* static code (IDEs) *or* runtime metrics (Activity Monitor). The Localhost Cartographer connects them. It knows that **PID 12345** *is* **Project X**, and that Project X *defines* **Endpoint Y**, which is currently returning **500 Errors**.

## Persistence Strategy
- **Memory Graph:** Stores the relational structure and historical uptime.
- **Filesystem:** Writes a human-readable `README_LIVE.md` in the root workspace, acting as an always-up-to-date map of the environment.

## Autonomy Level
- **High (Observation):** Runs autonomously in the background.
- **Low (Action):** Does not kill or start processes without explicit user confirmation (Safety First).

## Failure Modes & Recovery
- **Permission Denied:** Some processes (root) are invisible. Agent marks them as "System/Privileged".
- **Transient Processes:** Short-lived scripts might be missed. Agent focuses on long-running servers.
- **Drift:** If a process dies, the graph updates on the next sweep (10-30s interval).

