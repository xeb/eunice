# Codebase Archaeologist: Proactive Design

## Purpose
An agent that proactively identifies code quality issues, documentation gaps, and architectural drift—surfacing concerns before they become problems. Acts as a "code conscience" that maintains situational awareness.

## Core Tools
- **grep**: Pattern detection, code smell hunting
- **filesystem**: Structure analysis, documentation checking
- **memory**: Historical knowledge, drift detection
- **shell**: Running linters, tests, static analysis
- **web**: Checking for known vulnerabilities, best practices

## Loop Structure
```
DAILY PATROL:
1. SCAN for changes (git log since last patrol)
2. For each changed file:
   a. RUN quick analysis (complexity, size, patterns)
   b. CHECK against historical baseline
   c. DETECT drift (style, architecture, patterns)
   d. FLAG anomalies for review
3. HUNT for code smells:
   a. Files changed frequently (hotspots)
   b. Large files / complex functions
   c. TODO/FIXME/HACK comments
   d. Outdated dependencies
4. CHECK documentation:
   a. README freshness vs code changes
   b. Undocumented public APIs
   c. Broken links in docs
5. GENERATE report:
   a. New concerns
   b. Resolved concerns
   c. Trending issues
6. UPDATE memory with findings

WEEKLY DEEP DIVE:
1. SELECT one area for thorough review
2. ANALYZE architecture against best practices (web search)
3. COMPARE against similar projects
4. GENERATE improvement recommendations
```

## Memory Architecture
```
Entities:
- Concern (description, severity, location, status[new|acknowledged|resolved])
- Baseline (metric, value, measured_at, component)
- Trend (metric, direction, rate_of_change)
- Recommendation (description, effort, impact, status)

Relations:
- Concern -[LOCATED_IN]-> Component
- Baseline -[FOR]-> Component
- Trend -[TRACKS]-> Baseline
- Recommendation -[ADDRESSES]-> Concern
```

## Detection Patterns

### Complexity Drift
- Track cyclomatic complexity per file over time
- Alert when complexity grows faster than features

### Hotspot Detection
- Files changed > 10 times in 30 days
- Cross-reference with bug reports/fixes

### Documentation Rot
- README mentions features that don't exist
- Code comments reference old behavior
- Links to moved/deleted files

### Architectural Erosion
- Dependency direction violations
- Unexpected cross-module imports
- Pattern inconsistency

## Alert Levels

| Level | Meaning | Action |
|-------|---------|--------|
| INFO | Interesting observation | Include in report |
| WATCH | Potential concern | Track over time |
| WARN | Actionable issue | Highlight in report |
| ALERT | Significant problem | Immediate notification |

## Output

### Daily Report
```markdown
# Codebase Patrol: 2025-01-15

## New Concerns
- [WARN] auth/validator.js complexity increased 40% (3 changes this week)
- [INFO] New TODO added in api/routes.js: "// TODO: rate limiting"

## Trending
- [WATCH] test coverage decreasing: 82% -> 79% over 2 weeks

## Resolved
- [RESOLVED] package.json had outdated lodash (fixed in commit abc123)
```

## Failure Modes

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Alert fatigue | Human ignores reports | Reduce sensitivity, prioritize |
| False positives | Human dismisses alerts | Learn from dismissals |
| Missed issues | Bug found that agent missed | Add detection pattern |
| Analysis too slow | Timeout | Sample rather than full scan |

## Human Touchpoints
- **Report review**: Daily/weekly digest
- **Alert acknowledgment**: Mark concerns as accepted/resolved
- **Sensitivity tuning**: Adjust thresholds
- **Pattern teaching**: Add new detection rules
