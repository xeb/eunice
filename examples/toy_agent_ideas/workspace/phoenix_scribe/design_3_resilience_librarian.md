# Design 3: The Resilience Librarian (Hybrid)

## Purpose
To build a "Resilience Knowledge Graph" that connects past failures to code patterns, acting as a proactive guardian against recurring architectural mistakes.

## Loop Structure
1. **Knowledge Extraction**: Read all historical PMs. Extract entities (Services, Dependencies, Error Types) and Relations (caused_by, fixed_by, mitigated_via).
2. **Graph Construction**: Build a `memory` graph.
   - `[Entity: Redis] --(has_failure_mode)--> [Observation: Cache Stampede]`
   - `[Observation: Cache Stampede] --(prevented_by)--> [Pattern: Jittered Backoff]`
3. **Pattern Scanning**: Periodically scan the codebase () for "Risky Patterns" (e.g., `retries=3` without `backoff`).
4. **Contextual Warning**: When a developer touches a "Risky" area, search the Graph.
   - "Warning: You are modifying `auth_service`. In 2023, this service failed due to [Incident #42](link) because of a missing timeout. Please verify you have handled timeouts."

## Tool Usage
- **memory**: The core component. Stores the graph of failures and patterns.
- **grep**: Finds code that matches the "signatures" of past failures.
- **filesystem**: Reads code and PMs.

## Memory Architecture
- **Graph Database**: Heavy reliance on `memory_create_entities` and `memory_create_relations` to map the "Genealogy of Failure".

## Failure Modes
- **False Alarms**: Flagging every loop as a potential "Infinite Loop" incident. *Recovery:* User feedback loop ("Ignore this") updates the graph weights.
- **Stale Knowledge**: Old incidents might no longer be relevant.

## Human Touchpoints
- **Curation**: Humans verify the links in the graph.
- **Consultation**: Developers query the agent: "What has gone wrong with Redis before?"
