# Agent: The Simulation Architect (The "Mock Data Demigod")

## Purpose
To transform local development and testing environments from empty wastelands into thriving, realistic ecosystems. The **Simulation Architect** creates and manages a persistent population of "Synthetic Users" who live, age, and interact within the database, ensuring developers always have realistic, privacy-compliant, and edge-case-rich data for testing.

## Core Toolset
- **memory:** Stores the "Biography" of every synthetic user, their relationships, and the history of their actions.
- **filesystem:** Reads application schemas (SQL/Prisma) to understand the "laws of physics" (constraints) and writes seed files.
- **fetch:** Acts as the interface for the synthetic users to interact with the application APIs.
- **web:** Researches realistic content formats (addresses, bio text, product reviews) to avoid "Lorem Ipsum" blindness.

## Architecture

### 1. The World Model (Memory Graph)
The agent maintains a graph database representing the *intended* state of the world:
- **Entities:** `Persona` (e.g., "Angry Karen", "Power User"), `DataPoint` (generated IDs), `Constraint` (from schema).
- **Relations:** `Persona -> HAS_ACCOUNT -> UserID`, `Persona -> PREFERS -> Category`.
- **Knowledge:** It remembers *why* a user exists (e.g., "Created to test abandoned cart logic").

### 2. The Execution Loop (The "Life Cycle")
- **Birth (Schema Analysis):**
    - Agent scans `schema.prisma` or SQL files.
    - Updates its internal model of required fields and constraints.
    - Spawns new Personas if population density is low.
- **Life (Simulation):**
    - **Wake Up:** Agent iterates through active Personas.
    - **Decide Action:** Based on Persona traits (e.g., "Impulsive"), chooses an action (Buy, Return, Comment).
    - **Execute:** Uses `fetch` to hit the local API, just like a real frontend.
    - **Sleep:** Schedules next action (e.g., "Wait 3 days for shipping").
- **Entropy (Chaos Injection - Optional):**
    - Occasionally injects "Mutation" events: invalid Unicode, massive payloads, or SQL injection strings to test resilience.

### 3. Persistence Strategy
- **Hybrid Approach:**
    - **Memory:** Stores the *narrative* (Who is User 123?).
    - **Database:** Stores the *state* (User 123's actual rows).
    - **Filesystem:** Exports `seed.json` snapshots so the team can share specific interesting states (e.g., "The 'Big Data' state").

## Key Insight
**Test Data as a Living Process, not a Static Artifact.**
Instead of a static SQL dump that gets stale immediately, the Simulation Architect is a **Daemon** that "plays" the application in the background. If you add a new feature (e.g., "Wishlists"), the agent notices the new table and starts using it, naturally populating it with data without manual intervention.

## Failure Modes & Recovery
- **Schema/API Mismatch:** If the agent tries to call an endpoint that changed, it catches the 404/500 error.
    - *Recovery:* It triggers a "Research" phase, scanning the codebase/OpenAPI spec to relearn the endpoint, then updates its internal strategy.
- **Runaway Population:** Creating too much data.
    - *Recovery:* Implements a "Grim Reaper" logic to archive or delete old/inactive synthetic users to keep DB size manageable.

## Human Touchpoints
- **Demigod Mode:** Developers can issue commands like "Create 50 angry users who return items" to test specific flows.
- **Turing Verification:** Humans periodically check if the generated text/behavior looks natural.
