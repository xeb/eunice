# Knowledge Gardener: Final Design

## Synthesis

After exploring conservative, experimental, and collaborative approaches, the optimal design combines:
- **From Conservative**: Bounded autonomy, mandatory logging, human audit trails
- **From Experimental**: Confidence scoring, contradiction detection, pattern emergence
- **From Collaborative**: Inbox/outbox async pattern, natural language uncertainty

## The Knowledge Gardener

A **semi-autonomous knowledge curation agent** that maintains a growing knowledge graph while respecting human oversight through an asynchronous review pattern.

### Core Philosophy
The garden metaphor is intentional: like a gardener, this agent tends to knowledge—planting new facts, pruning outdated ones, noticing when topics need attention, and presenting the harvest for the human to enjoy and verify.

## Architecture

### Tool Usage
| Tool | Role | Frequency |
|------|------|-----------|
| memory | Primary knowledge store | Every cycle |
| web | Discovery of new information | 3-5 searches/cycle |
| fetch | Deep retrieval of sources | 1-3 fetches/cycle |
| filesystem | Inbox/outbox + snapshots | Every cycle |
| text-editor | Preparing digests | Weekly |

### Memory Schema
```
Entities:
- Topic {name, priority[1-5], freshness_days, last_tended}
- Fact {content, status[candidate|confirmed|disputed], confidence[0-1], discovered_at}
- Source {url, title, credibility[low|medium|high], fetched_at}

Relations:
- Topic -[CONTAINS]-> Fact
- Fact -[FROM]-> Source
- Fact -[CONTRADICTS]-> Fact (with strength and status)
```

### Filesystem Structure
```
workspace/knowledge_gardener/
├── inbox/
│   ├── approvals.json
│   ├── rejections.json
│   └── requests.md
├── outbox/
│   ├── candidates/
│   ├── digest.md
│   └── alerts.md
├── snapshots/
│   └── YYYY-MM-DD.json
└── logs/
    └── activity.md
```

## The Loop

```
EVERY CYCLE (default: daily):

1. WAKE
   - Log timestamp
   - Load current memory graph state

2. PROCESS INBOX
   - Apply human approvals (candidate -> confirmed)
   - Archive rejections with reason
   - Add new human questions to queue

3. ASSESS GARDEN
   - Identify topics needing attention:
     * Stale (last_tended > freshness_days)
     * Sparse (few confirmed facts)
     * Disputed (unresolved contradictions)
   - Prioritize by topic priority × staleness

4. TEND (for top 3 topics):
   a. Formulate search queries
   b. Execute web search
   c. Fetch promising URLs
   d. Extract candidate facts
   e. Check for duplicates/contradictions
   f. Add to memory as candidates

5. SYNTHESIZE
   - Look for cross-topic patterns
   - Detect new contradictions
   - Note emerging connections

6. PREPARE OUTBOX
   - Write candidate digest for human review
   - Flag contradictions requiring attention
   - Generate weekly summary (if weekly boundary)

7. SNAPSHOT
   - Export memory graph to filesystem
   - Rotate old snapshots (keep 30 days)

8. LOG & SLEEP
   - Record activity summary
   - Calculate next wake time
```

## Safeguards

### Bounded Autonomy
- Maximum 5 web searches per cycle
- Maximum 10 new candidate facts per cycle
- No deletion of confirmed facts (only archival)
- All changes logged with reasoning

### Human Checkpoints
| Event | Human Action Required |
|-------|----------------------|
| New candidate facts | Review and approve/reject |
| Detected contradiction | Resolve or acknowledge |
| Pattern suggestion | Validate or dismiss |
| Weekly digest | Acknowledge receipt |

### Recovery
- Daily filesystem snapshots enable rollback
- Memory operations are logged with undo information
- Inbox/outbox provides clear audit trail
- Failed cycles don't lose state (resume from last good state)

## Future Extensions

1. **Multi-human collaboration**: Multiple inboxes for team knowledge bases
2. **Source reputation learning**: Track which sources produce approved vs rejected facts
3. **Query strategy evolution**: Learn which search formulations yield best facts
4. **Topic suggestion**: Propose new topics based on discovered connections
5. **Integration hooks**: Export to Obsidian, Notion, or other knowledge tools

## Success Metrics

- **Coverage**: Facts per topic over time
- **Freshness**: Average fact age vs freshness requirements
- **Accuracy**: Approval rate of candidate facts
- **Efficiency**: Facts confirmed per search query
- **Engagement**: Human review session frequency
