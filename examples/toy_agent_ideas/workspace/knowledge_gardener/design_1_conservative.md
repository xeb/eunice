# Knowledge Gardener: Conservative Design

## Purpose
An agent that maintains and grows a personal knowledge graph by periodically searching the web for updates on topics the user cares about, then integrating new information into a structured memory graph.

## Core Tools
- **memory**: Primary persistence layer for the knowledge graph
- **web**: Brave search for discovering new information
- **fetch**: Retrieving full content from interesting URLs
- **filesystem**: Storing raw sources and generating exports

## Loop Structure
```
1. WAKE (scheduled or triggered)
2. READ current memory graph to understand existing knowledge
3. IDENTIFY topics that need refreshing (oldest last-updated)
4. SEARCH web for each topic (limited to 3-5 searches per wake)
5. FETCH promising URLs for deeper reading
6. EXTRACT key facts, entities, and relationships
7. COMPARE against existing memory (avoid duplicates)
8. CREATE new entities/relations for novel information
9. ADD observations linking sources to facts
10. LOG activity to filesystem for human review
11. SLEEP until next scheduled wake
```

## Memory Architecture
```
Entities:
- Topic (name, description, last_updated, priority)
- Fact (content, confidence, source_url, discovered_at)
- Source (url, title, fetched_at, reliability_score)

Relations:
- Topic -[HAS_FACT]-> Fact
- Fact -[SOURCED_FROM]-> Source
- Topic -[RELATED_TO]-> Topic
- Fact -[SUPPORTS|CONTRADICTS]-> Fact
```

## Failure Modes

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Web search fails | Empty results | Skip topic, try next wake |
| Fetch timeout | Exception handling | Log failure, continue with other URLs |
| Memory corruption | Validation on read | Export backup, alert human |
| Duplicate detection fails | Periodic human review | Manual deduplication tool |

## Human Touchpoints
- **Initial Setup**: Human defines initial topics of interest
- **Weekly Review**: Human reviews log of changes, can approve/reject
- **Priority Adjustment**: Human can boost/demote topic priorities
- **Quality Audit**: Periodic review of fact accuracy

## Constraints (Conservative)
- Maximum 5 web searches per wake cycle
- Maximum 10 new facts per wake cycle
- Mandatory 24-hour cooling period per topic
- All changes logged for human audit
- No automatic deletion of existing facts
