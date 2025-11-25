# Codebase Archaeologist: Conversational Design

## Purpose
A codebase understanding agent designed to answer developer questions in real-time by maintaining a searchable, reasoned knowledge base that it can consult and extend during conversation.

## Core Tools
- **grep**: Real-time code search
- **filesystem**: File reading and structure analysis
- **memory**: Persistent knowledge graph
- **text-editor**: Preparing responses

## Loop Structure (Conversational Mode)
```
ON QUESTION RECEIVED:
1. PARSE question intent:
   - What/where: Location query
   - Why: Rationale query
   - How: Implementation query
   - Who: Ownership query
2. SEARCH memory for existing knowledge
3. If knowledge found:
   - CHECK if still current (file timestamps)
   - RETURN answer with confidence
4. If knowledge gap:
   a. GREP for relevant code
   b. READ promising files
   c. ANALYZE and form hypothesis
   d. RECORD new knowledge to memory
   e. RETURN answer, marked as "newly discovered"
5. SUGGEST follow-up questions
```

## Memory as Conversation Context
```
Entities:
- Concept (name, definition, confidence, last_verified)
- Location (path, purpose, related_concepts[])
- Explanation (question, answer, sources[], created_at)

Relations:
- Concept -[IMPLEMENTED_AT]-> Location
- Explanation -[REFERENCES]-> Concept
- Concept -[RELATED_TO]-> Concept
```

## Question Types & Strategies

### "Where is X?"
1. Grep for X (class name, function, string)
2. Return locations with context
3. Store mapping in memory

### "Why does X do Y?"
1. Check memory for prior explanations
2. Read X's implementation
3. Read git history for X
4. Form hypothesis, present with confidence
5. Invite human to confirm/correct

### "How does X work?"
1. Read X's code
2. Grep for X's usages
3. Build mental execution model
4. Generate explanation with code excerpts
5. Create/update Concept entity

### "Who knows about X?"
1. Git blame X's files
2. Aggregate by author
3. Cross-reference with past questions about X
4. Return ranked list of experts

## Learning Loop
```
AFTER CONVERSATION:
1. Review questions asked
2. Identify knowledge gaps exposed
3. For high-value gaps:
   a. Deep-dive investigation
   b. Record comprehensive explanation
4. Suggest documentation improvements
```

## Failure Modes

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Can't find answer | Empty grep + memory | Admit uncertainty, ask human |
| Wrong answer given | Human correction | Update memory, decrease confidence |
| Outdated answer | Timestamp mismatch | Re-investigate, mark old answer |
| Too many results | Grep returns 100+ | Ask for clarification, narrow scope |

## Human Touchpoints
- **Every answer**: Human can correct or confirm
- **Confidence calibration**: Human rates answer quality
- **Priority hints**: "Focus on understanding X"
- **Teaching moments**: Human explains something, agent records
