# Design 3: The "Council" (Hybrid/Multi-Persona)

## Purpose
To avoid hallucination and bias, this agent simulates a "committee" of diverse viewpoints before committing knowledge to the graph. It turns the "internal monologue" of an LLM into an explicit "external dialogue" between personas.

## Loop Structure (Dialectical Batch)
1. **Topic Selection**: Pick a subject entity from the existing graph.
2. **Data Gathering**: Run a neutral web search to gather raw context.
3. **The Council Session**:
   - **Persona A (The Believer)**: Generates arguments *for* the entity's importance/truth, extracts maximum connections.
   - **Persona B (The Skeptic)**: Critiques the sources, checks for bias, looks for debunking evidence.
   - **Persona C (The Historian)**: Checks `memory_search_nodes` for past context or cyclical patterns.
4. **Debate Synthesis**:
   - The agent writes a "Meeting Minutes" document (Markdown) where these personas "talk" (simulated).
   - They vote on the "Truth Value" of the new information.
5. **Commit**:
   - If Consensus > Threshold: Write to `memory` (Graph).
   - If Consensus < Threshold: Write to `filesystem` (quarantine folder) as a "Disputed Fact".
6. **Publication**:
   - Generates a "Newsletter" summarizing the debates, not just the facts.

## Tool Usage
- **memory**: Stores only high-confidence, vetted knowledge.
- **filesystem**: Stores the "Minutes" (rich context, nuance, dissent). The filesystem acts as the "unconscious" or "raw" memory, while the Graph is the "conscious" crystallized knowledge.
- **web**: Used by all personas, potentially with different search queries (Skeptic searches for "X scam", Believer searches for "X breakthrough").

## Memory Architecture
- **Bicameral**:
  - **Graph**: Clean, sparse, high-truth.
  - **Filesystem**: Messy, verbose, dialectical.
- **Links**: Graph nodes contain file paths to the specific "Meeting Minutes" where that entity was discussed.

## Failure Modes
- **Gridlock**: The personas never agree. ADDRESSED BY: A "Tiebreaker" rule (defaults to excluding from Graph, keeping in Filesystem).
- **Echo Chamber**: All personas share the same underlying model bias. ADDRESSED BY: Forcing varied "System Prompts" for each persona during the generation phase.

## Human Touchpoints
- **Judge**: Human can view the "Disputed Facts" folder and manually resolve/commit them.
- **Observer**: Human reads the "Meeting Minutes" to understand *why* the agent believes what it believes.
