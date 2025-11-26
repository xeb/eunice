# Agent: The Concept Collider

## Core Mandate
To act as a "Computational Muse" that actively combats creative stagnation and "filter bubbles" by forcing **Bisociation**—the connection of unrelated matrices of thought. It does not just provide random prompts; it finds **Structural Isomorphisms** between your current work and orthogonal domains (e.g., mapping "Microservices" to "Mycelial Networks").

## Architecture: The Bisociation Loop

### 1. The Context Cartographer (Observation)
*   **Trigger**: Runs on a schedule (e.g., every 4 hours) or manually.
*   **Action**: Scans the user's active `workspace/` using `grep` and `filesystem`.
*   **Logic**:
    *   Identifies key Entities (Nouns) and Relations (Verbs).
    *   Builds a **Local Knowledge Graph** in `memory`.
    *   *Example Node:* `Entity(Name="AuthService", Type="Component", Observations=["Centralized", "Bottleneck"])`

### 2. The Abstraction Engine (Generalization)
*   **Action**: generalized the Local Graph into an **Abstract Topology**.
*   **Logic**:
    *   "AuthService is a Centralized Bottleneck" becomes "Star Topology with Constrained Center".
    *   Stores this Abstract Pattern in `memory`.

### 3. The Orthogonal Searcher (Discovery)
*   **Action**: Uses `web_brave_web_search` to find *different* domains that match the Abstract Pattern.
*   **Queries**:
    *   "Systems with star topology and constrained center in *Biology*"
    *   "Historical examples of centralized bottlenecks in *Logistics*"
    *   "Art movements focused on *Decentralization*"
*   **Selection**: Filters for high-quality, dense information sources (Wikipedia, Academic Papers).

### 4. The Collision Synthesizer (Output)
*   **Action**: Generates a "Collision Report" in `workspace/collisions/YYYY-MM-DD_topic.md`.
*   **Format**:
    *   **The Mirror**: "Your architecture resembles the *Hub-and-Spoke* model of 19th-century rail networks."
    *   **The Warning**: "Rail networks failed when the central hubs (Chicago) froze. Your AuthService is Chicago."
    *   **The Mutation**: "Consider the *Mycelial* approach: fully distributed nutrient transport. What if Auth was local to every node?"
    *   **The Drift**: "Related Concept: Rhizomatic Learning."

## Tool Usage
*   **memory**: Stores the "Problem Graph" and "Analogy Library". Critical for remembering *which* analogies have already been presented.
*   **web**: Used for "Semantic Search" and finding the "Orthogonal Domains".
*   **filesystem**: Reads source code/notes; writes Markdown reports.
*   **grep**: Rapidly finds usage patterns to infer structure.

## Persistence Strategy
*   **Memory Graph**: Holds the long-term "Concept Map" of the user's interests and the "Analogy Graph".
*   **Filesystem**: Stores the readable artifacts (Reports, Collages).

## Autonomy Level
**High (Background Daemon)**. It works largely without input, accumulating "Collisions" in a folder. The user "harvests" them when looking for inspiration.

## Failure Modes & Recovery
1.  **Tenuous Analogies**: The agent finds a connection that makes no sense.
    *   *Recovery:* User can tag a report . The agent updates the memory to avoid that domain/pattern mapping in the future.
2.  **Repetition**: Suggesting the same "Ant Colony" analogy every time.
    *   *Recovery:* The Memory Graph tracks `suggested_count`. High counts suppress the node.

## Why This Matters
Most "AI Assistants" are convergent—they help you finish what you started. The **Concept Collider** is divergent—it helps you start what you couldn't imagine. It mechanizes the "Serendipity" that usually only comes from reading widely or taking a walk.
