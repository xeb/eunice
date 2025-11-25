# Agent Design: The Narrative Loom

## Abstract
**The Narrative Loom** is an autonomous intelligence agent designed to weave scattered news events into coherent, evolving narrative graphs. Unlike standard news aggregators that provide static snapshots, the Loom maintains a persistent **Temporal Knowledge Graph** of events, actors, and causal relationships. It detects **narrative shifts** (e.g., when a "protest" evolves into a "revolution") and generates reports that explain *how* a story has changed over time, highlighting conflicting perspectives rather than flattening them into a single truth.

## Core Architecture

### 1. The Loom Loop (Execution Cycle)
The agent operates in a continuous or scheduled loop (e.g., every 4 hours):

1.  **Ingest (The Spindle):** 
    *   Polls `web_brave_news_search` for tracked "Story Arcs".
    *   Fetches full text via `fetch_fetch` for deep analysis.
2.  **Graph Grafting (The Weaving):**
    *   Extracts Entities and Events.
    *   Queries `memory_search_nodes` to find existing context.
    *   Updates the Memory Graph:
        *   **New Events** are linked to **Previous Events** via `CAUSED` or `FOLLOWED` edges.
        *   **Claims** are linked to **Sources**.
3.  **Pattern Detection (The Pattern Check):**
    *   Analyzes the graph for topological changes:
        *   **Forking:** One event leading to two contradictory outcomes/reports.
        *   **Merger:** Disparate storylines converging.
        *   **Acceleration:** A sudden density of events in a short timeframe.
4.  **Narrative Synthesis (The Cloth):**
    *   Generates a Markdown report explaining the "Delta" in the graph.
    *   "Since yesterday, Node A (The Bill) has moved from 'Proposed' to 'Stalled', caused by Node B (Veto Threat)."

### 2. Tool Usage Strategy
*   **memory (The Weaver's Memory):** 
    *   *Nodes:* `Event`, `Actor` (Person/Org), `Claim`, `Source`.
    *   *Edges:* `PRECEDES`, `CAUSES`, `ASSERTED`, `DISPUTES`.
    *   *Observations:* Used to store raw text snippets or evidence supporting a link.
*   **web (The Eyes):**
    *   Uses `web_brave_news_search` to find fresh inputs.
    *   Uses `web_brave_search` to resolve entity ambiguity (e.g., "Which 'Smith' is this?").
*   **filesystem (The Archive):**
    *   Stores the generated narrative reports in `reports/YYYY-MM-DD_<topic>.md`.
    *   Stores a visual export of the graph (e.g., DOT/Mermaid) for human inspection.

## Persistence Strategy
*   **Primary:** The **Memory Graph** is the source of truth. It allows the agent to survive restarts and recall the state of a story from months ago.
*   **Secondary:** The **Filesystem** acts as the "Publication Layer," creating human-readable artifacts that persist outside the agent's internal memory.

## Autonomy & Human-in-the-Loop
*   **Autonomy:** High. The agent autonomously gathers, connects, and synthesizes.
*   **Human Checkpoints:**
    *   **Topic Seeding:** Humans must define the initial "Narrative Arc" (e.g., "Track the AI Regulation Bill").
    *   **Graph Pruning:** Humans can intervene to merge duplicate nodes or sever incorrect causal links if the agent hallucinates a connection.

## Failure Modes & Recovery
*   **Narrative Drift:** The agent might start tracking irrelevant side-stories.
    *   *Recovery:* Uses a "Relevance Score" decay. Nodes not connected to the main "Core Entities" effectively wither and are ignored in reports.
*   **Contradiction Paralysis:** When two trusted sources say opposite things.
    *   *Recovery:* The agent does not try to resolve the truth. It creates a `DISPUTE` node linking the two conflicting Claims, making the conflict *part of the story*.

## Key Insight
Most agents try to answer "What happened?". The Narrative Loom answers **"How did the story change?"**. By explicitly modeling time and causality in a graph, it captures the *evolution* of reality, which is often more valuable than the raw facts themselves.
