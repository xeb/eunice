# Final Design: The Chronicler

## Executive Summary
**The Chronicler** is an autonomous "Living History" engine. It continuously monitors a domain (e.g., "AI Regulation"), extracts factual entities into a Knowledge Graph, and weaves them into human-readable "Story Arcs". Uniquely, it uses **Meta-Cognition Nodes** to store its own reasoning within the graph, making its "thought process" debuggable and observable.

## Core Architecture
### 1. The Ingestion Loop (Daily)
- **Action:** Triggers at 6:00 AM.
- **Tools:** `web_brave_news_search` (fetch), `fetch_fetch` (deep dive).
- **Process:**
  1. Fetch top 20 news items.
  2. Extract Entities (Who/What) and Events (Did What).
  3. **Dedup:** Check `memory_search_nodes` to see if this event is already recorded (fuzzy matching).

### 2. The Reasoning Layer (Meta-Memory)
- **Innovation:** Instead of just A -> B, the agent stores:
  - Node A (Event 1)
  - Node B (Event 2)
  - Node C (Reasoning): "I linked A and B because both mention 'Section 230' and occur within 24h."
  - Edge: A -> linked_to -> C -> justifies -> B.
- **Benefit:** Allows humans to audit *bad links* by deleting the "Reasoning Node", effectively severing the causal chain.

### 3. The Narrative Layer (Filesystem)
- **Action:** After graph updates, the agent scans for "Active Story Arcs".
- **Process:**
  - If a new Event connects to an existing Arc (via graph cluster), append a paragraph to `workspace/stories/arc_ID.md`.
  - If an Event is orphaned but significant, create new `workspace/stories/new_arc_ID.md`.
- **Tools:** `text-editor` (append mode), `filesystem_write_file` (new arcs).

## MCP Toolchain
| Tool | Purpose |
|------|---------|
| **web** | Raw material (News/Search). |
| **memory** | The "Brain". Stores Entities, Events, and *Reasoning*. |
| **filesystem** | The "Journal". Stores human-readable narratives (Markdown). |
| **grep** | Pattern matching across the Narrative files to find contradictions. |

## Data Structure
- **Nodes:** `Entity`, `Event`, `StoryArc`, `Reasoning`.
- **Edges:** `PARTICIPATED_IN`, `CAUSED`, `CONTRADICTS`, `BELONGS_TO_ARC`.

## Failure Recovery
- **Hallucination:** If the agent invents a link, the "Reasoning Node" will likely be weak or nonsensical. A "Auditor" script (or human) can query for "Reasoning Nodes" with low confidence scores and prune them.
- **Staleness:** A weekly cron job archives Story Arcs that haven't been touched in 30 days to `archive/`.

## Roadmap
1. **MVP:** Single domain (e.g., "Rust Programming Language"). Fixed schema.
2. **Phase 2:** Multi-domain support.
3. **Phase 3:** "Reader Agent" that answers user questions based *only* on the Markdown narratives.
