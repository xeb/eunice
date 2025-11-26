# Design 3: The Ghost Librarian (Knowledge Graph)

## Purpose
A "Knowledge Retrieval" agent that treats external links as *imports* into your personal knowledge graph. It downloads the content, but its primary goal is not just storage—it is **indexing**. It extracts entities, summaries, and full text so you can search *your bookmarks* locally using semantic search, even if you never tagged them.

## Loop Structure
1. **Ingest**: Watch for new links.
2. **Download & Distill**:
    - Download page content (`fetch`).
    - Convert to plain text (`shell`/`pandoc`).
    - Summarize and extract keywords/entities (`memory`/`llm`).
3. **Graph Construction**:
    - Create `WebPage` entities in `memory`.
    - Link `WebPage` to concepts mentioned in your local notes.
4. **Search Interface**:
    - Agent creates a `_search_index.md` or exposes a CLI tool.
    - User asks: "Where did I read about 'CRDTs'?"
    - Agent checks memory graph and finds the archived URL content that discusses CRDTs, even if the user's note just said `[Link]`.

## Tool Usage
- **memory**: Heavy usage. Storing the graph of "Page Content" linked to "Local Context".
- **filesystem**: Storing the raw text/media for full retrieval.
- **grep**: Used to search the raw text archives.

## Memory Architecture
- **Entities**: `WebPage`, `Concept`, `Author`.
- **Relations**: `WebPage mentions Concept`, `Note cites WebPage`.
- **Observations**: "Page Summary", "Key Quotes".

## Failure Modes
- **Information Overload**: Graph becomes too noisy with irrelevant web entities. -> Apply strict relevance filtering (only entities also present in local notes).
- **Stale Content**: The web page changes, but the graph has old info. -> Re-crawl periodically.

## Human Touchpoints
- **Query**: The primary interaction is the user asking questions.
- **Curation**: User can delete "useless" pages from the graph.
