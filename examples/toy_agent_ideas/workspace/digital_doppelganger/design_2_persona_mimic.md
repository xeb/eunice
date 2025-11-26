# Design 2: The Persona Mimic (Curated Masks)

## Purpose
To dilute the user's profile not with noise, but with *competing valid signals*. By masquerading as different distinct demographic archetypes, it forces ad networks to categorize the user as "Everything," rendering targeting expensive and ineffective.

## Loop Structure
1. **Select Persona**: Agent picks a persona from its Memory Graph (e.g., "The Doomsday Prepper").
2. **Context Load**: Retrieves the "Interest Graph" for that persona (nodes: 'Canned Food', 'Bunkers', 'Ham Radio').
3. **Session**:
    - Searches for a topic from the graph.
    - Reads a news article (Web Search + Summarizer).
    - "Learns" a new related topic and adds it to the Persona's graph.
4. **Context Switch**: After 20 mins, wipes state and switches to "The K-Pop Superfan".

## Tool Usage
- **memory_search_nodes**: To find the current persona's interests.
- **memory_add_observations**: To expand the persona's depth (making it look "alive" and evolving).
- **web_brave_web_search**: To generate traffic.

## Memory Architecture
- **Graph-Based**: Each Persona is a central node.
- **Relations**: `(Persona: Prepper) -> INTERESTED_IN -> (Topic: Solar Generators)`.
- **Evolution**: The personas grow over time. The Prepper might get into "Hydroponics" naturally via graph traversal.

## Failure Modes
- **Semantic Drift**: A persona might drift into illegal or flagged content if not bounded.
- **Recovery**: "Reset" a persona to its seed state if it gets too weird.

## Human Touchpoints
- **Definition**: User defines the initial 5 personas to ensure they don't overlap with real interests.

## Critique
Highly effective but resource-intensive. Requires maintaining complex graphs.
