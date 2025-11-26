# Design 2: The Actor (Active Simulator)

## Purpose
A fully autonomous "Stateful Digital Twin" of an external API. Instead of just replaying static responses, it reads the API documentation (OpenAPI/Swagger) and *generates code* to simulate the logic (CRUD operations, state changes) so the mock behaves realistically over time.

## Loop Structure
1. **Initialize:** Reads `openapi.yaml` from the filesystem.
2. **Research:** If the spec is incomplete, uses `web` to search for "API documentation for [Service]" to understand behavior (e.g., "How does Stripe handle partial refunds?").
3. **Model:** Creates a "State Machine" in the **Memory Graph**.
   - *Example:* "User" entity has properties [id, name, email].
4. **Generate:** Writes a Python/FastAPI script (via `filesystem`) that implements the endpoints.
   - *Crucial:* The script uses the `memory` tool as its database!
   - `POST /users` -> calls `memory_create_entities`.
   - `GET /users/{id}` -> calls `memory_search_nodes`.
5. **Serve:** Boots the generated server.
6. **Refine:** If the server throws errors, the Agent reads the stack trace and patches the code.

## Tool Usage
- **filesystem:** Read specs, write server code (`mock_server.py`).
- **web:** Research edge cases and error codes.
- **memory:** Acts as the *runtime database* for the mock server, allowing persistent state across restarts.
- **shell:** Run the server, install dependencies.

## Memory Architecture
- **Meta-Memory:** Stores the "Logic" of the API (how endpoints relate).
- **Data-Memory:** Stores the *actual data* created during testing (the specific users/items created by `POST` requests).

## Failure Modes
- **Hallucination:** The agent infers incorrect logic (e.g., assumes DELETE is soft-delete).
- **Complexity:** Cannot mock complex server-side business logic (e.g., image processing, payment clearing).
- **Recovery:** Human can edit the generated `mock_server.py` manually.

## Human Touchpoints
- **Review:** User reviews the generated mock logic.
- **Override:** User can manually inject state into Memory to set up test scenarios.
