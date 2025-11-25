# Knowledge Gardener: Collaborative Design

## Purpose
A knowledge management agent that operates as a collaborative partner rather than an autonomous system—designed for tight human-AI knowledge curation where the agent proposes and the human decides.

## Core Tools
- **memory**: Shared knowledge graph (human + agent contributions)
- **web**: On-demand search triggered by knowledge gaps
- **fetch**: Content retrieval for human review
- **filesystem**: Inbox/outbox pattern for async collaboration
- **text-editor**: Preparing knowledge for human review

## Loop Structure
```
1. CHECK inbox (filesystem) for human requests/feedback
2. PROCESS feedback:
   - Approved facts: Promote to confirmed
   - Rejected facts: Archive with reason
   - New questions: Add to investigation queue
   - Topic adjustments: Update priorities
3. INVESTIGATE one item from queue:
   a. Search and gather candidate facts
   b. Score by relevance and novelty
   c. Prepare digest with recommendations
4. PREPARE outbox:
   - New fact candidates for approval
   - Questions for human clarification
   - Weekly knowledge summary
5. WAIT for next human interaction or scheduled check
```

## Memory Architecture
```
Entities:
- Topic (name, human_interest_score, agent_recommendations)
- Fact (content, status[candidate|approved|rejected|archived])
- Question (text, priority, asked_by[human|agent], status)
- Session (timestamp, human_feedback[], agent_actions[])

Relations:
- Topic -[HAS_CANDIDATE]-> Fact (agent proposed)
- Topic -[HAS_CONFIRMED]-> Fact (human approved)
- Human -[ASKED]-> Question
- Agent -[SUGGESTED]-> Question
- Fact -[ADDRESSES]-> Question
```

## Inbox/Outbox Pattern

### Inbox (Human -> Agent)
```
workspace/knowledge_gardener/inbox/
├── approve_facts.json      # Fact IDs to confirm
├── reject_facts.json       # Fact IDs with rejection reasons
├── new_questions.txt       # Free-form questions to investigate
├── topic_updates.json      # Priority changes, new topics
└── feedback.md             # General guidance
```

### Outbox (Agent -> Human)
```
workspace/knowledge_gardener/outbox/
├── candidates/             # Fact proposals with evidence
│   ├── 2025-01-15_topic_ai.md
│   └── 2025-01-15_topic_climate.md
├── questions.md            # Agent's questions for human
├── weekly_digest.md        # Summary of knowledge changes
└── alerts.md               # Contradictions, stale info, concerns
```

## Collaboration Modes

### Synchronous (Human Present)
Agent responds immediately to queries, fetches on demand, presents options for human decision.

### Asynchronous (Human Away)
Agent works through investigation queue, prepares batched recommendations, waits for human return.

### Review Session
Human goes through outbox, agent explains reasoning, rapid approve/reject flow.

## Failure Modes

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Inbox backlog | File count threshold | Summarize and prioritize |
| Human unresponsive | Days since last feedback | Pause investigations, send alert |
| Recommendation fatigue | High rejection rate | Reduce candidate volume, ask why |
| Topic stagnation | No activity on topic | Suggest archiving or refresh |

## Human Touchpoints
- **Every fact**: Must be human-approved before confirmation
- **Weekly review**: Digest of agent activity
- **Monthly calibration**: Adjust agent aggressiveness
- **On-demand**: Human can trigger immediate investigation

## Collaborative Features

### Confidence as Conversation
Instead of numeric confidence, agent expresses uncertainty in natural language:
- "I'm fairly sure about this, found in 3 reputable sources"
- "This might be outdated, the newest source is from 2022"
- "Sources disagree on this—want me to dig deeper?"

### Teaching Moments
When human rejects a fact, agent asks why to improve future recommendations:
- "Was this irrelevant, inaccurate, or already known?"

### Shared Annotation
Both human and agent can add notes to facts, creating a dialogue around the knowledge.
