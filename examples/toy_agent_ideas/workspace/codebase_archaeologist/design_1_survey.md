# Codebase Archaeologist: Survey Design

## Purpose
An agent that continuously explores and documents a codebase, building a living understanding of architecture, patterns, dependencies, and institutional knowledge that can answer questions about "why is the code this way?"

## Core Tools
- **grep**: Pattern searching across codebase
- **filesystem**: Reading files, understanding structure
- **memory**: Building knowledge graph of code concepts
- **shell**: Running analysis tools (git log, dependency analysis)

## Loop Structure
```
1. WAKE (triggered by changes or scheduled)
2. DETECT changes since last run (git diff, file timestamps)
3. For each changed area:
   a. READ affected files
   b. GREP for related patterns
   c. ANALYZE: What changed? Why might it have changed?
   d. UPDATE memory graph with new understanding
4. EXPLORE one under-documented area:
   a. Select area with lowest documentation coverage
   b. READ and analyze files
   c. GREP for usage patterns
   d. BUILD mental model of component
   e. RECORD findings to memory
5. GENERATE documentation artifacts:
   a. Architecture diagrams (mermaid)
   b. Pattern documentation
   c. Dependency maps
6. SLEEP
```

## Memory Architecture
```
Entities:
- Component (name, path, purpose, complexity_score)
- Pattern (name, description, locations[], why_used)
- Decision (description, rationale, affected_components[])
- Person (name, areas_of_expertise[]) -- from git blame
- Question (text, status[open|answered], answer)

Relations:
- Component -[DEPENDS_ON]-> Component
- Component -[USES_PATTERN]-> Pattern
- Component -[AUTHORED_BY]-> Person
- Decision -[AFFECTS]-> Component
- Question -[ABOUT]-> Component
```

## Key Investigations
- **"Why does this exist?"** - Trace git history, find original commit message
- **"Who knows about this?"** - Git blame analysis
- **"What depends on this?"** - Grep for imports/usage
- **"What patterns are used here?"** - Compare against known patterns

## Output Artifacts
```
workspace/codebase_archaeologist/
├── maps/
│   ├── architecture.md (mermaid diagrams)
│   ├── dependencies.md
│   └── ownership.md (who knows what)
├── patterns/
│   └── <pattern_name>.md
├── decisions/
│   └── <decision_log>.md
└── qa/
    └── questions.md (open questions + answers)
```

## Failure Modes

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Codebase too large | Timeout on grep | Incremental exploration, sampling |
| Git history missing | No commits found | Rely on code structure analysis |
| Circular dependencies | Graph cycle detection | Mark and document |
| Outdated knowledge | Code changed without agent run | Re-analyze affected areas |

## Human Touchpoints
- **Question answering**: Human asks, agent searches its knowledge
- **Decision documentation**: Human explains why, agent records
- **Pattern naming**: Agent identifies pattern, human names it
- **Priority guidance**: Human indicates important areas to focus on
