# Codebase Archaeologist: Final Design

## Synthesis

The optimal design combines three operational modes that can be used independently or together:
- **Survey Mode** (from Design 1): Background exploration and documentation
- **Conversational Mode** (from Design 2): Real-time question answering
- **Patrol Mode** (from Design 3): Proactive issue detection

## The Codebase Archaeologist

A **multi-modal code understanding agent** that builds and maintains a living knowledge base about a codebase, capable of answering questions, surfacing concerns, and generating documentation.

### Core Philosophy
Codebases are like archaeological sites—layers of decisions, patterns, and context that take time to understand. This agent is the resident archaeologist: always learning, always documenting, always ready to explain what it has discovered.

## Architecture

### Tool Usage
| Tool | Role | Mode |
|------|------|------|
| grep | Pattern search, usage finding | All modes |
| filesystem | File reading, structure analysis | All modes |
| memory | Knowledge persistence | All modes |
| shell | Git analysis, running tools | Survey, Patrol |
| web | Best practices, vulnerability checks | Patrol |
| text-editor | Report generation | Survey, Patrol |

### Memory Schema
```
Entities:
- Component {path, purpose, complexity_score, coverage_score, last_analyzed}
- Pattern {name, description, locations[], rationale}
- Concern {description, severity, status, location, detected_at}
- Explanation {question, answer, sources[], confidence, created_at}
- Person {name, email, areas_of_expertise[], last_active}

Relations:
- Component -[DEPENDS_ON]-> Component
- Component -[USES]-> Pattern
- Component -[HAS_CONCERN]-> Concern
- Explanation -[ABOUT]-> Component
- Person -[EXPERT_IN]-> Component
```

### Filesystem Structure
```
workspace/codebase_archaeologist/
├── knowledge/
│   ├── components/
│   │   └── <component_path_hash>.md
│   ├── patterns/
│   │   └── <pattern_name>.md
│   └── decisions/
│       └── <decision_id>.md
├── reports/
│   ├── daily/
│   └── weekly/
├── qa/
│   ├── answered/
│   └── open/
└── config/
    ├── ignore_patterns.txt
    └── thresholds.json
```

## Operational Modes

### Mode 1: Survey (Background)
```
TRIGGER: Scheduled or on-demand
LOOP:
1. Git log for changes since last survey
2. For each changed component:
   - Analyze changes
   - Update component knowledge
   - Detect pattern usage
3. Select one unexplored area
4. Deep dive: read, analyze, document
5. Update memory and generate artifacts
```

### Mode 2: Conversational (Interactive)
```
TRIGGER: Human question
LOOP:
1. Parse question type
2. Search memory for existing answer
3. If found and current: return with confidence
4. If gap: investigate (grep, read, analyze)
5. Form answer, record to memory
6. Return answer with sources
7. Invite follow-up or correction
```

### Mode 3: Patrol (Watchdog)
```
TRIGGER: Daily schedule
LOOP:
1. Scan for changes
2. Run detection patterns:
   - Complexity growth
   - Hotspots
   - Documentation drift
   - TODO accumulation
3. Compare against baselines
4. Generate concern list
5. Produce daily report
6. Update memory with findings
```

## Integration Points

### Git Integration
- Pre-commit: Quick patrol of staged changes
- Post-merge: Survey new code from branches
- History mining: Understand "why" from commit messages

### CI/CD Integration
- Export metrics for tracking dashboards
- Produce documentation artifacts for publishing
- Generate architecture diagrams for PRs

### IDE Integration
- Answer questions inline
- Show component knowledge on hover
- Surface concerns in problem panel

## Human Collaboration

### Teaching the Archaeologist
- Human can explain decisions → agent records
- Human can name patterns → agent tracks usage
- Human can dismiss concerns → agent learns thresholds

### Getting Answers
- Ask natural language questions
- Get confidence-rated answers
- See sources and reasoning

### Reviewing Findings
- Daily/weekly patrol reports
- Trend visualizations
- Actionable recommendations

## Safeguards

### Bounded Operations
- Maximum grep results: 100 per query
- Maximum file reads: 50 per session
- Timeout per analysis: 30 seconds

### Knowledge Hygiene
- All knowledge has timestamps
- Confidence decays without re-verification
- Stale knowledge flagged for refresh

### Human Override
- Any agent belief can be corrected
- Any concern can be dismissed
- Any pattern can be renamed/redefined

## Success Metrics

- **Coverage**: % of codebase with documented knowledge
- **Freshness**: Average age of knowledge vs code changes
- **Accuracy**: Human correction rate on answers
- **Value**: Time saved answering questions
- **Health**: Trend of detected concerns (should decrease over time)

## Future Extensions

1. **Multi-repo awareness**: Track patterns across repositories
2. **Team knowledge**: Learn which humans know what
3. **Onboarding mode**: Guided tour for new developers
4. **Architecture enforcement**: Block PRs that violate patterns
5. **Historical storytelling**: Generate "history of feature X"
