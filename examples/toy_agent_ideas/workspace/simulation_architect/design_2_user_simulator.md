# Design 2: The User Simulator

## Purpose
To move beyond static data rows and generate "Behavioral Data" — a database state that reflects realistic user activity over time, revealing bugs related to state transitions, caching, and data aging.

## Loop Structure
1. **Persona Selection:** Pick a "Synthetic User" from memory (e.g., "Indecisive Shopper").
2. **Session Execution:**
   - Log in via API/Headless Browser.
   - Perform a sequence of actions (View Item -> Add to Cart -> Wait 2 Days -> Remove Item).
3. **Observation:** Check if the application state matches expectations (e.g., "Is the cart empty?").
4. **Sleep/Schedule:** Schedule the next interaction for this persona (simulating real time gaps).

## Tool Usage
- **fetch:** To interact with the application APIs directly.
- **memory:** To maintain the state and history of every synthetic user (their "memories").
- **web:** To generate realistic content (forum posts, reviews) so the app looks organic.
- **shell:** To run headless browsers for UI-only flows.

## Memory Architecture
- **Nodes:** `Persona`, `Session`, `Goal`.
- **Edges:** `EXECUTED`, `ACHIEVED`, `FAILED`.
- **Persistence:** The memory graph acts as the "Brain" for the synthetic users, tracking what they "know" vs. what is actually in the database.

## Failure Modes
- **API Drift:** API endpoints change, breaking the simulation. *Recovery:* Agent reads API docs or OpenAPI specs to self-heal its request builders.
- **State Desync:** The agent thinks it added an item, but the server errored. *Recovery:* Agent performs a "re-sync" probe to check actual server state before continuing.

## Human Touchpoints
- **Scenario Definition:** Humans define high-level goals (e.g., "Simulate a Black Friday traffic spike").
- **Turing Test:** Humans can inspect generated content to ensure it looks realistic enough for demos.
