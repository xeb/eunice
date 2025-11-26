# Design 2: The Isomorphic Mirror

## Purpose
To discover "Structural Isomorphisms" (shared patterns) between the user's current problem domain and unrelated fields (Biology, History, Physics) to provide deep architectural insights rather than just surface-level prompts.

## Loop Structure
1. **Model**:
   - Periodically scan the user's `workspace/` code and notes.
   - Extract "Abstract Relationships" (e.g., "Centralized control with distributed workers", "Resource scarcity", "Conflict resolution").
   - Store these in the **Memory Graph** as a "Problem Topology".
2. **Search**:
   - Query `web_brave_web_search` for these abstract patterns in *other* domains.
   - *Query Example:* "Biological systems with centralized control and distributed workers" or "Historical examples of resource scarcity in siege warfare".
3. **Bisociate**:
   - Compare the found external system with the internal problem.
   - Identify "Solutions" the external system uses (e.g., "Ants use pheromones to decentralize").
4. **Synthesize**:
   - Write a "Mirror Report" to `workspace/insights/isomorph_report.md`.
   - "Your distributed queuing system resembles the supply lines of the Roman Legions. Consider how they handled 'packet loss' (ambushes) via redundancy..."

## Tool Usage
- **memory**: Heavy use. Stores the "Problem Topology" (Nodes: Components, Edges: Relations).
- **web**: Complex queries to find analogical domains.
- **grep/filesystem**: To extract the structure of the current codebase.

## Memory Architecture
- **Graph-based**:
  - `Entity(Type="SystemComponent", Name="LoadBalancer")`
  - `Relation(From="LoadBalancer", To="Worker", Type="DistributesTo")`
  - `Observation(Entity="LoadBalancer", Content="Single point of failure")`
- **Analogies**:
  - `Entity(Type="Analogy", Name="QueenBee")`
  - `Relation(From="LoadBalancer", To="QueenBee", Type="IsomorphOf")`

## Failure Modes
- **Hallucination**: Finding analogies that don't exist or are tenuous. (Mitigation: User feedback loop "Rate this analogy").
- **Complexity**: The graph becomes too noisy. (Mitigation: Prune old nodes, focus on high-level architecture).

## Human Touchpoints
- **Review**: User reviews the "Mirror Reports".
- **Steer**: User can suggest a domain ("Find analogies in Jazz Music").
