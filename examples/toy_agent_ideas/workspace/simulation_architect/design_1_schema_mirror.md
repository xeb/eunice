# Design 1: The Schema Mirror

## Purpose
To automatically generate and maintain static test data seeds that are guaranteed to be compliant with the current database schema, eliminating "broken seed" errors during CI/CD.

## Loop Structure
1. **Watch Mode:** Monitor `schema.sql` or ORM definitions (e.g., Prisma, TypeORM, Django models) for changes.
2. **Analyze:** When a change is detected, parse the new structure to identify required fields, types, and constraints.
3. **Generate:** Create synthetic data rows that satisfy these constraints.
4. **Persist:** Update the `seed.json` or `seed.sql` files used by the development team.

## Tool Usage
- **filesystem:** Read schema files; Write seed files.
- **grep:** Quickly identify changed lines in schema files.
- **shell:** Execute local database validation commands (e.g., `npm run db:validate`).
- **memory:** Store mapping rules (e.g., "Field 'email' requires regex format X").

## Memory Architecture
- **Nodes:** `Table`, `Column`, `Constraint`.
- **Edges:** `HAS_COLUMN`, `REFERENCES` (foreign keys).
- **Persistence:** Uses the memory graph to map abstract concepts (like "Customer") to technical implementations, ensuring data consistency across table renames.

## Failure Modes
- **Complex Constraints:** Custom database triggers might reject data that looks valid statically. *Recovery:* Agent parses the SQL error message and adjusts the generation rules.
- **Circular Dependencies:** Tables referencing each other. *Recovery:* Agent detects cycles and inserts data in specific transactional order or uses deferred constraints.

## Human Touchpoints
- **Review:** PRs created by the agent with updated seed data.
- **Configuration:** Humans can tag fields with semantic types (e.g., "@is:phone_number") if the agent cannot infer them.
