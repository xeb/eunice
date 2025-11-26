# Design 3: The Narrative Weaver (Hybrid)

## Purpose
To bridge the gap between raw data and human storytelling. This agent focuses on **Causality Chains**. It doesn't just know *what* happened, but attempts to link events into "Story Arcs" (e.g., "The Rise and Fall of Startup X").

## Loop Structure
1. **Event Detection:** Monitors news like Design 1.
2. **Causal Linking:**
   - When Event B happens, it queries the graph for Event A such that "A caused B" or "B references A".
   - Uses LLM inference to assign a confidence score to this causality.
3. **Arc Management:**
   - **Active Arcs:** Maintains a list of "Open Storylines".
   - **Update:** Appends new events to relevant arcs.
   - **Closure:** If an arc has no activity for 30 days, it generates a "Post-Mortem" markdown file and archives the arc.
4. **Publication:** Generates a static documentation site (e.g., MkDocs) where each page is a Story Arc.

## Tool Usage
- **memory:** Stores atomic facts.
- **filesystem:** Stores the "Narrative Layer" (Markdown files). The graph points to file paths.
- **text-editor:** Appends new paragraphs to existing Story Arc files.

## Memory Architecture
- **Hybrid Storage:**
  - **Graph:** Fast lookup of entities and rigid relationships.
  - **Files:** Rich text narratives, summaries, and "The Story So Far".
- **Arc Entities:** The graph contains "Story Arc" nodes that connect to "Event" nodes.

## Failure Modes
- **False Causality:** Linking unrelated events ("Ice cream sales caused Shark attacks").
- **Narrative Bloat:** Stories never ending.
- **Mitigation:** "Editor Bot" sub-routine that summarizes and truncates long files every week.

## Human Touchpoints
- **Editorial Review:** Humans can "close" a story arc manually or merge two arcs.
- **Curator:** Humans select which Arcs are published to the front page.
