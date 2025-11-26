# Design 3: The Cross-Pollinator (Hybrid)

## Purpose
A "Diplomatic Translator" that helps users from one technical culture (e.g., JavaScript) understand and communicate with another (e.g., Rust) by finding **Analogies** and **Translation Layers**.

## Loop Structure
1.  **Dual-Scanning:** Agent scans two communities simultaneously (Source & Target).
2.  **Ontology Mapping:**
    *   Identifies concepts in Target that map to Source.
    *   *Example:* "Cargo.toml" (Target) ~= "package.json" (Source).
3.  **Gap Analysis:** Identifies concepts *without* direct mappings (e.g., "Borrow Checker" has no JS equivalent).
4.  **Bridge Generation:**
    *   Creates a "Rosetta Stone" markdown file.
    *   Translates the user's "Mental Model" queries (e.g., "How do I npm install?") into the Target's vernacular ("cargo add").
5.  **Maintenance:** Continuously updates the mapping as tools evolve.

## Tool Usage
*   **Memory:** Stores two distinct graphs connected by `ANALOGOUS_TO` edges.
*   **Web:** Searches for "X vs Y" articles to seed the analogy graph.
*   **Filesystem:** Maintains a dynamic glossary/cheatsheet.

## Memory Architecture
*   **Bridge Nodes:**
    *   (Concept: Promise [JS]) --[ANALOGOUS_TO]--> (Concept: Future [Rust])
    *   (Edge Attribute: "Accuracy: 80%") - notes nuance differences.

## Failure Modes
*   **False Equivalence:** Suggesting a mapping that is technically wrong (e.g., "Class == Struct"). *Recovery:* Agent adds "Nuance Notes" to edges when it finds contradictions online.

## Human Touchpoints
*   **Querying:** User asks "What is the X for Y?"
*   **Correction:** User corrects bad analogies, strengthening the graph.
