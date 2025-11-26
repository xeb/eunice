# Design 2: The Digital Muse (Innovative)

## Purpose
A proactive "Creative Companion" that reads your drafts (stories, game design docs) and autonomously builds a "Visual Context" graph, surrounding your work with inspiration you didn't explicitly ask for but likely need.

## Loop Structure
1. **Analyze**: Periodically scan target project folders (e.g., `my_novel/chapters/`) using `filesystem` and `grep`.
2. **Extract**: Identify key themes, emotional words, and setting descriptions (e.g., "rain-slicked neon", "abandoned hospital").
3. **Graph Expansion**:
   - Query **Memory** to see if these concepts exist.
   - If not, create entities and link them (e.g., `Entity("Hospital") --[attribute]--> Entity("Abandoned")`).
   - Use `web_brave_web_search` to find "aesthetic synonyms" (e.g., "liminal spaces", "brutalist decay") and add them to the graph.
4. **Curate**:
   - Perform `web_brave_image_search` for the expanded terms.
   - Download the best matches.
5. **Present**:
   - Instead of just folders, generate a `moodboard.html` file in the project root.
   - This file displays a masonry grid of images linked to the specific chapters/files they relate to.

## Tool Usage
- **memory**: Stores the "Aesthetic Ontology" (mapping words to visual vibes).
- **grep**: Scans user content for keywords.
- **web**: Both web search (for concepts) and image search (for assets).
- **filesystem**: Reads user content, writes the HTML dashboard.

## Memory Architecture
- **Graph-Based**:
  - Nodes: Concepts ("Cyberpunk"), Emotions ("Melancholy"), Files ("chapter1.md").
  - Edges: `INSPIRED_BY`, `EVOKES`, `VISUALLY_SIMILAR_TO`.
  - Allows the agent to "remember" that "chapter1.md" has a "Cyberpunk" vibe, so if the user adds a "Cyberpunk" character later, it suggests existing images.

## Failure Modes
- **Hallucination**: Extracting irrelevant keywords (e.g., "the" or "and").
  - *Mitigation*: Use a "Stoplist" of common words in Memory.
- **Context Drift**: The moodboard becomes cluttered.
  - *Recovery*: Prune "weak" links in the memory graph over time (forgetting mechanism).

## Human Touchpoints
- **Implicit**: The user just writes their story/code. The agent reacts.
- **Explicit**: The user can "Ban" a concept from the `moodboard.html` interface (which writes back to a `banlist.txt` the agent reads).
