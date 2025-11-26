# Design 3: The Semantic Localhost Cartographer

## Purpose
A persistent background agent that maps the *meaning* of local runtime state. It doesn't just manage processes; it connects them to code, documentation, and dependencies to answer "Why is this running?" and "How do I interact with it?".

## Loop Structure
1. **Discovery:**
   - Scan active ports/processes.
   - Trace PID -> CWD -> Git Repo -> `package.json`.
2. **Enrichment (The "Semantic" part):**
   - **Tech Stack:** Identify "It's a Python/FastAPI app".
   - **Endpoints:** Parse source code (grep) to find `/health`, `/api/v1`.
   - **Dependencies:** Parse `docker-compose.yml` to see "This service needs Postgres on port 5432".
3. **Graphing:**
   - Build a Knowledge Graph: `Service(Auth) -> OWNS -> Port(3000)`, `Service(Auth) -> DEPENDS_ON -> Service(Postgres)`.
4. **Interface:**
   - Expose a CLI query interface: `agent ask "What relies on the auth service?"`.
   - Expose a "Context File": `workspace/localhost_map.md` (Live documentation).

## Tool Usage
- **memory:** Graph database to store the topology (Process <-> Code <-> Port).
- **filesystem:** Deep inspection of config files and source code.
- **grep:** Finding API routes and config variables.
- **web:** Looking up default ports (e.g., "What usually runs on 6379? Redis.").

## Memory Architecture
- **Entities:** `Process`, `Port`, `Repository`, `Endpoint`, `Database`.
- **Relations:** `LISTENING_ON`, `CONNECTED_TO`, `DEFINED_IN`.

## Failure Modes
- **Stale Graph:** Process dies, but graph remains until next scan.
- **Misidentification:** Confusing two node services.

## Human Touchpoints
- **Query:** User asks questions about their environment.
- **Annotation:** User adds notes to the graph ("Port 3000 is the OLD backend").

