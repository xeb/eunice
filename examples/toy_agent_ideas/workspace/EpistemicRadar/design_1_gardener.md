# Design 1: The "Gardener" (Conservative)

## Purpose
The Gardener is a low-risk, high-reliability agent designed to maintain an up-to-date Knowledge Graph (KG) of a specific, pre-defined technology domain (e.g., "Rust Web Frameworks"). It focuses on accuracy and provenance over speed or breadth.

## Loop Structure (Scheduled Batch)
1. **Wake Up**: Triggered by a cron job or external scheduler (e.g., once every 24 hours).
2. **Load Schema**: Reads a strict schema definition from `workspace/EpistemicRadar/schema.json` (allowed entity types and relation types).
3. **Scan Sources**: Iterates through a curated list of reliable URL sources (defined in `sources.txt`).
4. **Extract & Verify**:
   - For each source, uses `web_brave_web_search` to check for updates (e.g., "latest release of Actix").
   - Extracts potential entities/relations.
   - **Verification Step**: Must cross-reference with a second source before accepting a new fact.
5. **Update Graph**:
   - Uses `memory_add_observations` to log the evidence.
   - Uses `memory_create_entities` only if the entity fits the schema.
   - Updates `last_checked` timestamp on existing nodes.
6. **Report**: Generates a daily changelog in `workspace/EpistemicRadar/reports/daily_YYYY-MM-DD.md`.
7. **Sleep**: Terminates process.

## Tool Usage
- **memory**: Strictly typed usage. Only creates entities found in `schema.json`.
- **web**: Restricted to specific domains or high-trust queries.
- **filesystem**: Reads config, writes logs. No code modification.

## Memory Architecture
- **Graph**: Serves as the "ground truth" state.
- **Observations**: Used as an audit trail. Every edge in the graph must link back to a specific observation ID.

## Failure Modes
- **Schema Mismatch**: If a new important concept appears that fits no type, the agent logs an "Anomaly" to a file and skips it. It does not hallucinate new types.
- **Source Down**: Skips and retries next cycle.
- **Conflict**: If Source A says X and Source B says Not X, it flags the node as "Contested" and alerts the human via the report.

## Human Touchpoints
- **Schema updates**: Human must manually edit `schema.json` to broaden scope.
- **Conflict Resolution**: Human reviews "Contested" nodes.
