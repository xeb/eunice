# Design 3: The Margins (Hybrid)

## Purpose
To create a "Shadow Documentation" layer that exists *alongside* the official text. Instead of editing the source files (which might be frozen or owned by a specific team), this agent maintains a sidecar database of "Annotations" that map to specific hashes of document paragraphs. It's like a communal "Genius.com" annotation layer for technical docs.

## Loop Structure
1.  **Ingest:** Read all official docs and hash each paragraph.
2.  **Link:** Scan the web for discussions. When a discussion cites a specific doc page or function name, create a link.
3.  **Annotate:** Store the relationship: `DocParagraph_Hash -> [Community_Discussion_1, Community_Discussion_2]`.
4.  **Render:** Generate a static site mirror (or a browser extension overlay JSON) that shows the official docs *plus* the community margins.

## Tool Usage
*   `memory_create_entities`: To store the `Paragraph` nodes and `Discussion` nodes.
*   `memory_create_relations`: To link `Paragraph --annotated_by--> Discussion`.
*   `filesystem_read_file`: To process the source docs.
*   `web_brave_web_search`: To find external references.

## Memory Architecture
*   **Fine-Grained Graph:** Nodes are not just files, but *blocks of text*. This allows the agent to handle file restructuring (re-hashing to find where the paragraph moved).
*   **Drift Detection:** If a paragraph changes significantly (hash change), the annotations are flagged as "Orphaned" and need re-linking.

## Failure Modes
*   **Desynchronization:** The official docs change, and the annotations point to nowhere. *Recovery:* Fuzzy matching algorithms to re-attach annotations to the most similar new paragraph.
*   **Overload:** Too many annotations make the docs unreadable. *Mitigation:* UI/UX design to collapse margins by default.

## Human Touchpoints
*   **Reader:** The primary interaction is passive reading of the "Enriched" docs.
*   **Feedback:** Users can upvote/downvote the *relevance* of an annotation to a specific paragraph.
