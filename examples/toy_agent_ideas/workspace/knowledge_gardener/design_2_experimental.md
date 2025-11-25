# Knowledge Gardener: Experimental Design

## Purpose
A self-evolving knowledge system that not only gathers information but actively seeks to identify gaps, contradictions, and emerging connections in its knowledge graph—then autonomously investigates to resolve them.

## Core Tools
- **memory**: Knowledge graph with confidence scoring
- **web**: Multi-strategy search (web, news, academic)
- **fetch**: Deep content retrieval with summarization
- **filesystem**: Versioned knowledge snapshots
- **shell**: Running analysis scripts on gathered data

## Loop Structure
```
1. WAKE
2. ANALYZE current graph for:
   - Knowledge gaps (topics with few facts)
   - Stale information (facts older than threshold)
   - Contradictions (conflicting facts)
   - Weak links (low-confidence relations)
3. PRIORITIZE investigation targets by impact score
4. For each target (up to N):
   a. FORMULATE search queries (multiple angles)
   b. SEARCH using appropriate strategy (news for current, web for general)
   c. FETCH and SUMMARIZE content
   d. EXTRACT structured information
   e. RECONCILE with existing knowledge:
      - Update confidence scores
      - Mark contradictions for review
      - Create provisional facts (pending confirmation)
5. SYNTHESIZE: Look for emergent patterns across topics
6. EXPORT: Generate human-readable knowledge digest
7. REFLECT: Log what worked/failed for self-improvement
8. SLEEP
```

## Memory Architecture
```
Entities:
- Topic (name, importance, freshness_requirement, investigation_count)
- Fact (content, confidence[0-1], status[confirmed|provisional|disputed])
- Source (url, credibility_score, fetch_history)
- Investigation (query, timestamp, yield_score)
- Pattern (description, supporting_facts[], confidence)

Relations:
- Topic -[HAS_FACT {confidence}]-> Fact
- Fact -[CONTRADICTS {strength}]-> Fact
- Fact -[SUPPORTS]-> Pattern
- Investigation -[DISCOVERED]-> Fact
- Topic -[INFLUENCED_BY]-> Topic
```

## Novel Mechanisms

### Confidence Decay
Facts have confidence that decays over time based on topic freshness requirements. A fact about "AI research" decays faster than one about "historical events."

### Contradiction Resolution
When contradictions are detected:
1. Both facts marked as disputed
2. Agent formulates specific queries to investigate
3. Additional sources either resolve or deepen the dispute
4. Human escalation if unresolvable after N attempts

### Pattern Emergence
Agent periodically runs "synthesis" pass looking for:
- Multiple facts that suggest unstated connections
- Topics that should be linked but aren't
- Clusters of related information that could be a new topic

## Failure Modes

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Runaway investigation | Token/time budget exceeded | Hard stop, log state |
| Confidence collapse | Too many facts below threshold | Snapshot + alert human |
| Topic drift | Semantic similarity check | Prune distant concepts |
| Echo chamber | Source diversity metric | Force novel source search |

## Human Touchpoints
- **Bootstrap**: Initial topic seeding
- **Contradiction Escalation**: Unresolvable disputes
- **Pattern Validation**: Confirm emergent patterns
- **Confidence Calibration**: Periodic accuracy audit
- **Budget Approval**: Large investigation campaigns

## Experimental Features
- Self-adjusting wake schedule based on topic activity
- Automatic topic suggestion based on discovered connections
- Source credibility learning from human feedback
- Investigation strategy evolution (which queries work best)
