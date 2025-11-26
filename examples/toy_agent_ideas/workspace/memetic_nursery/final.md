# Agent Design: The Memetic Nursery

## Abstract
**The Memetic Nursery** is a background agent that acts as an incubator for the user's "Idea Backlog." Unlike traditional project management tools that treat backlog items as static tasks to be sorted, the Memetic Nursery treats them as living "Seeds" that require specific conditions (Context, Feasibility, Market Timing) to germinate. It autonomously "waters" these seeds with web research, checks their technical feasibility with micro-prototypes, and puts them into "Cryostasis" when blocked, only waking them when the external environment changes (e.g., a new API is released).

## Core Philosophy
**"Ideas are not to be done; they are to be grown."**
Most ideas fail not because they are bad, but because the timing is wrong. This agent solves the *Timing* problem by decoupling "Ideation" from "Execution" via a persistent, autonomous "Maturation" phase.

## Primary Toolset
1.  **Memory (Graph Database)**: Stores the "Genome" of each idea (dependencies, blockers, maturity score).
2.  **Web (Brave Search)**: Scans for external triggers (new libraries, competitor launches).
3.  **Filesystem**: Manages the human interface (Markdown files) and technical proofs (prototype folders).
4.  **Shell**: Executes "Tracer Bullet" scripts to verify technical assumptions.

## Architecture & Loop

### 1. Ingestion (Germination)
*   **Trigger**: User creates a file `seeds/idea-name.md` or adds an entry to `inbox.md`.
*   **Action**: The agent parses the text, extracts key entities (e.g., "Requires: Python, GPT-4, Vector DB"), and creates a Node in the Memory Graph.
*   **Status**: `SEEDLING`

### 2. Enrichment (Watering)
*   **Trigger**: Periodic (Daily/Weekly).
*   **Action**: The agent performs web searches for the extracted entities.
    *   *Competitor Check*: "Are there existing apps like this?"
    *   *Resource Check*: "Is there a library for X?"
*   **Output**: Appends a "Context & Research" section to the Markdown file.

### 3. Feasibility Testing (Pruning)
*   **Trigger**: When a technical dependency is identified.
*   **Action**: The agent attempts to verify the dependency.
    *   *Soft Check*: Search docs ("Does API X support feature Y?").
    *   *Hard Check*: Generate and run a 10-line script ("Try to import Library X and run function Y").
*   **Decision**:
    *   **Success**: Status -> `VIABLE`.
    *   **Failure**: Status -> `DORMANT`. The agent records *exactly* what failed (e.g., "Error: API lacks stream support") and adds a **Watch Condition** to the Memory Graph.

### 4. Cryostasis & Awakening
*   **Trigger**: Weekly Scan of DORMANT nodes.
*   **Action**: The agent re-checks the **Watch Condition**.
    *   *Example*: "Search for 'OpenAI API stream support release notes'".
*   **Awakening**: If the blocker is removed, the agent moves the status back to `SEEDLING`, notifies the user, and re-runs the Feasibility Test.

## Persistence Strategy
*   **Hybrid**:
    *   **Filesystem**: The "User Interface". Markdown files in `seeds/`, `incubator/`, and `graveyard/`.
    *   **Memory Graph**: The "Brain". Stores the complex relationships, watch conditions, and feasibility history that don't fit cleanly into a Markdown file.

## Failure Modes & Recovery
1.  **Hallucinated APIs**: The agent tries to use a library that doesn't exist.
    *   *Recovery*: The "Hard Check" (execution) will fail, correcting the agent's belief.
2.  **Infinite Loops in Prototypes**: Malicious or buggy generated code.
    *   *Recovery*: Strict 5-second timeout on all  executions. Network access restricted to whitelisted domains (pypi, npm, github).
3.  **Notification Fatigue**: "New competitor found!" every day.
    *   *Recovery*: Group updates into a "Weekly Digest" unless a `CRITICAL_BLOCKER_REMOVED` event occurs.

## Human Handoff
*   **The "Harvest"**: When an idea reaches `Maturity: 90%` (Feasible + Novel + Documented), the agent moves the file to `ready_to_build/` and generates a "Project Scaffolding" (package.json, README structure) for the user to pick up.
