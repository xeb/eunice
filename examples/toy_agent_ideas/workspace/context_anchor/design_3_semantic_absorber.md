# Design 3: The Semantic Absorber (Graph Integration)

## Purpose
Instead of just preserving the *file* (the container), this agent preserves the *meaning* (the content) by ingesting linked resources into a persistent Knowledge Graph. This allows the agent to answer questions based on external docs even if the link dies, effectively "learning" the internet context of the project.

## Loop Structure
1.  **Crawl:** Continuously monitor codebase for new links.
2.  **Extract:** Fetch the content of the link.
3.  **Process:**
    *   Use `fetch` to get text.
    *   (Simulated LLM) Summarize key concepts, definitions, and arguments.
    *   Extract entities (e.g., "Library X", "Algorithm Y").
4.  **Graph:**
    *   `memory_create_entities`: Create nodes for the Article, the Author, and the Concepts.
    *   `memory_create_relations`: Link the File in the codebase to the Concepts it references.
5.  **Augment:**
    *   Add a "Knowledge Card" to the memory graph.
    *   Can generate a glossary file `docs/glossary.md` that auto-fills with definitions from these external links.

## Tool Usage
*   **memory:** Primary storage. The "web" becomes part of the agent's long-term memory.
*   **fetch:** Extraction.
*   **web:** To find metadata (author, date) if not in the raw HTML.
*   **filesystem:** To update the `glossary.md` or `references.md`.

## Memory Architecture
*   **Graph Database:** Maps Code -> references -> URL -> contains -> Knowledge.
*   **Semantic Search:** Allows developers to ask "Where did we reference the 'paxos' algorithm?" and find both the code file and the external article that explains it.

## Failure Modes
*   **Drift:** If the external concept changes meaning (unlikely for static docs, possible for living docs), the graph becomes stale.
*   **Hallucination:** Summarization might miss the *specific* nuance the developer intended to reference.

## Human Touchpoints
*   **Curator Mode:** Humans review the extracted entities to ensure they are relevant.
