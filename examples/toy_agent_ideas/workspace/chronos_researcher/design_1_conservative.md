# Design 1: The Daily Archivist

## Purpose
A reliable, low-hallucination agent designed to maintain a factual ledger of events for a specific domain (e.g., "Semiconductor Industry" or "AI Regulations"). It focuses on precision and provenance.

## Loop Structure
1. **Trigger:** Scheduled daily execution (e.g., 06:00 AM).
2. **Input:** Fetches top 20 news items for the domain using `web_brave_news_search`.
3. **Extraction:**
   - Identifying distinct entities (Companies, People, Laws).
   - Extracting rigid triples (Entity A -> ACTION -> Entity B).
4. **Verification:** Cross-references new triples against existing Knowledge Graph (KG) to prevent duplicates.
5. **Commit:** Uses `memory_create_entities` and `memory_create_relations`.
6. **Reporting:** Generates a "Daily Diff" text file summarizing new nodes and edges.

## Tool Usage
- **web:** `web_brave_news_search` for raw data.
- **memory:** `memory_search_nodes` to check existence, `memory_create_*` to store.
- **filesystem:** Writes logs and reports to `daily_briefings/`.

## Memory Architecture
- **Schema:** Fixed. Defined in a config file (e.g., `schema.json`).
- **Strict Types:** only allows specific relations like `ACQUIRED`, `RELEASED`, `SUED`.
- **Provenance:** Every observation includes the source URL and timestamp.

## Failure Modes
- **Schema Rigidty:** New types of events (e.g., "Prompt Injection Attack") might not fit existing relations. Agent logs these as "Uncategorized" for human review.
- **Ambiguity:** If multiple entities look similar (e.g., "Apple" the fruit vs. the company), it asks for human disambiguation or skips.

## Human Touchpoints
- Reviewing "Uncategorized" events.
- Updating the `schema.json` to allow new relation types.
