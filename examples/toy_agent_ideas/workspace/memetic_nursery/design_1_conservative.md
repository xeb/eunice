# Design 1: The Idea Archivist (Conservative)

## Purpose
To solve the problem of "Idea Rot" where backlog items become stale, lose context, or are forgotten. The Idea Archivist acts as a diligent librarian, continuously enriching static backlog files with fresh context from the web.

## Loop Structure
1. **Scan**: Read all `*.md` files in `ideas/` directory.
2. **Extract**: Identify key concepts, technologies, and hypotheses using NLP.
3. **Research**: Perform Brave Web Searches for extracted terms to find recent news, libraries, or similar projects.
4. **Enrich**: Append a "Fresh Context" section to the file with summarized findings and links.
5. **Tag**: Update the file's metadata (YAML frontmatter) with a "Freshness Score" and "Related Topics".
6. **Sleep**: Wait for a defined interval (e.g., 24 hours).

## Tool Usage
- **filesystem**: Read idea files, write updates.
- **web_brave_web_search**: Find external context.
- **memory**: Store a graph of "Related Ideas" to link separate files together (e.g., "Idea A" and "Idea B" both rely on "Vector DBs").

## Memory Architecture
- **Filesystem-First**: The source of truth is the Markdown file in the `ideas/` folder.
- **Memory Graph**: Used only for indexing and cross-referencing.
    - Nodes: `Idea`, `Technology`, `Problem`
    - Edges: `USES`, `SOLVES`, `RELATED_TO`

## Failure Modes
- **Hallucination**: Appending irrelevant search results. *Recovery*: User can manually delete bad sections; Agent learns to filter better.
- **Link Rot**: Added links die. *Recovery*: Agent periodically checks links and flags dead ones (Digital Amber style).

## Human Touchpoints
- **Input**: User creates a file `ideas/my-cool-app.md`.
- **Review**: User reads the "Fresh Context" section.
- **Control**: User can add `ignore: true` to frontmatter to stop updates.
