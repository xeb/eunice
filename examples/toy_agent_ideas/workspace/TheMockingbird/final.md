# Final Design: The Mockingbird (Autonomous API Simulator)

## Purpose
**The Mockingbird** is an autonomous agent that solves the "Missing Dependency" problem in software development. It creates high-fidelity, stateful mock servers for third-party APIs (like Stripe, Twilio, or internal microservices) by reading their documentation/specs and using the Memory MCP as a persistent state store.

It moves beyond static "canned responses" by implementing actual CRUD logic, allowing developers to test complex flows (Create -> Read -> Update -> Delete) against a local simulation that costs nothing and never goes down.

## System Architecture

### 1. The Architect Loop (Design Phase)
*Trigger:* User drops an `openapi.yaml` into the `input/` folder or provides a URL.
1.  **Analysis:** The agent reads the spec using `filesystem`.
2.  **Enrichment:** It uses `web` to find nuances not in the spec (e.g., specific error message formats, rate limit headers).
3.  **Code Generation:** It writes a custom Python FastAPI server (`server.py`).
    *   *Innovation:* The generated code is instrumented to use the **Memory MCP** as its database.
    *   Instead of a local SQLite file, the mock server sends `call_tool("memory_create_entities")` to store data created by `POST` requests.
    *   This allows the *Agent* to inspect the mock's state and reason about it.

### 2. The Server Loop (Runtime Phase)
*Trigger:* `shell` command starts the generated server.
1.  **Listen:** Server accepts HTTP requests on localhost.
2.  **Logic:**
    *   **GET /resource/{id}:** Queries Memory Graph for node `id`.
    *   **POST /resource:** Creates new node in Memory Graph.
    *   **Logic Fallback:** If logic is undefined, queries the Agent (LLM) to "Improvise" a valid response based on the spec schema.
3.  **Chaos Injection (Optional):** Randomly injects latency or errors based on known failure modes found during research.

## Core Tool Usage

| Tool | Purpose |
|------|---------|
| **filesystem** | Reading API specs, writing the `server.py` implementation, logging interactions. |
| **memory** | **Dual Use:** 1. Stores the *Logic Model* (how endpoints relate). 2. Acts as the *Runtime Database* for the mock server (storing users, items, etc.). |
| **web** | Researching API behavior, error codes, and "undocumented features" to make the mock realistic. |
| **shell** | Executing the mock server, installing `fastapi`/`uvicorn`. |

## Memory Strategy: "The State Graph"

The Memory Graph is partitioned into two layers:

1.  **The Schema Layer (Static):**
    *   Entity: `Endpoint` (/v1/charges)
    *   Entity: `Schema` (ChargeObject)
    *   Relation: `Endpoint` RETURNS `Schema`

2.  **The State Layer (Dynamic):**
    *   Entity: `Resource:user_123`
    *   Observation: `{"name": "Alice", "balance": 500}`
    *   *This allows the Mockingbird to remember that you created "Alice" 5 minutes ago.*

## Failure Modes & Recovery

1.  **Logic Drift:** The mock implements logic differently than the real API (e.g., pagination starts at 0 vs 1).
    *   *Recovery:* User provides a "Correction" text file. Agent reads it and hot-patches `server.py`.
2.  **Performance:** The Memory MCP is slower than a real database.
    *   *Mitigation:* Mockingbird is for *functional* testing, not load testing.
3.  **Complex Logic:** Cannot mock server-side computations (e.g., generating a valid PDF invoice).
    *   *Fallback:* Returns a static placeholder asset marked "MOCK_ARTIFACT".

## Human Touchpoints
- **Spec Provisioning:** "Here is the Swagger file for the API I want mocked."
- **State Inspection:** "What users exist in the mock right now?" (Agent queries Memory).
- **Scenario Seeding:** "Create 50 users with overdue invoices in the mock." (Agent loops `memory_create_entities`).
