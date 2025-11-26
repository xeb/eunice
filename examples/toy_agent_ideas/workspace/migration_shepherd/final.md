# Agent: The Migration Shepherd

## Core Concept
A "Strangler Fig" automation agent that acts as a **runtime supervisor** for safe, incremental system migration. It combines offline "Golden Master" generation with online "Shadow Traffic" routing to mathematically prove parity between legacy and modern implementations before switching them over.

## Core Toolset
- **shell:** To spawn proxy servers, run applications, and execute test suites.
- **memory:** To build a persistent "Parity Graph" that tracks which inputs are safe to switch.
- **fetch:** To act as the network layer for probing and routing.
- **filesystem:** To generate shim code, logs, and snapshot files.

## Problem Domain
Rewriting legacy code (e.g., Python 2 -> 3, REST -> GraphQL, Monolith -> Microservice) is high-risk. Developers often guess if the new system works. This agent removes the guesswork by automating the "Parallel Run" pattern.

## Architecture

### 1. The Offline Phase (Behavioral Profiling)
* **Goal:** Establish a baseline.
* **Action:** Agent scans the codebase (using `grep`) to find entry points. It fuzzes them (using `shell`) and records inputs/outputs to the **Memory Graph**.
* **Insight:** Instead of static files, it stores observations as `LegacyObservation` nodes.

### 2. The Shadow Phase (Runtime Verification)
* **Goal:** Prove equivalence on live/mock data.
* **Action:** Agent spins up a local **Proxy Server**. It routes traffic to the Legacy system but asynchronously "shadows" it to the New system.
* **Logic:**
    - If `Legacy.response == New.response`: Create `ParityMatch` edge.
    - If `Legacy.response != New.response`: Create `Divergence` node with diff details.
* **Autonomy:** The agent actively **blocks** the New system from taking real traffic until the "Parity Score" for that specific endpoint > 99%.

### 3. The Cutover Phase (The Strangler)
* **Goal:** Replace the old system.
* **Action:** When confidence is high, the Agent modifies the Proxy configuration (or the code itself via `filesystem`) to make the New system the primary.
* **Safety:** It keeps the Legacy system as a "Fallback" for a user-defined cooldown period.

## Unique Value
Most coding agents just write code. The **Migration Shepherd** manages the *deployment and verification* of that code. It is an **Operational Agent**.

## Failure Handling
- **Non-Determinism:** The agent learns to ignore fields that always change (timestamps, UUIDs) by detecting high variance in the "Diff" analysis.
- **Side Effects:** Configurable "Safe Mode" where the Shadow system runs against a mock DB or read-only replica.

## Sample Command
`shepherd migrate --port 3000 --legacy ./old_server --candidate ./new_server --strategy shadow`
