# Research Agent

A multi-agent research system using eunice's orchestration and the built-in `search_query` tool with Gemini Google Search grounding.

Based on the **orchestrator-workers** and **evaluator-optimizer** patterns from [Building effective agents](https://www.anthropic.com/research/building-effective-agents).

## How It Works

1. **Lead agent** breaks your request into subtopics
2. Spawns **researcher subagents** using `search_query` with `pro_preview` model
3. Each researcher saves findings to `research_notes/`
4. Spawns **report-writer** to create report in `reports/`
5. **Evaluator** reviews the report (APPROVED or NEEDS_REVISION)
6. If revision needed, report-writer revises **once**

## Requirements

- Eunice installed (`cargo install eunice`)
- `GEMINI_API_KEY` environment variable set
- `mcpz` MCP server installed (`cargo install mcpz`)

## Usage

```bash
cd examples/research_agent

# Run with a direct prompt
eunice "What is the #1 office chair of 2025?"

# Or interactively
eunice -i
```

## Agents

| Agent | Role | Tools |
|-------|------|-------|
| root (lead) | Coordinates, delegates to subagents | `invoke_*` |
| researcher | Gathers info via web search | `search_query`, `filesystem_write_*` |
| report_writer | Synthesizes findings into reports | `filesystem_*` |
| evaluator | Reviews report quality | `filesystem_read_*`, `filesystem_list_*` |

## Example Session

```
> What is the #1 office chair of 2025?

  root→researcher Identify the #1 rated office chair...
  🔧 search_query
→ [Detailed findings from Wirecutter, Tom's Guide, PCMag...]
  🔧 filesystem_write_file
→ Successfully wrote to research_notes/best_office_chair_2025.md

  root→report_writer Synthesize the research notes...
  🔧 filesystem_read_file
  🔧 filesystem_write_file
→ Successfully wrote to reports/office_chair_2025_summary.md

  root→evaluator Review the report...
  🔧 filesystem_read_file
  VERDICT: APPROVED
  STRENGTHS:
  - Clearly identifies top contenders with trade-offs
  - Includes specific pricing and features
  - Covers multiple budget segments

Complete. Report: reports/office_chair_2025_summary.md
```

## Output

- `research_notes/` - Individual research notes
- `reports/` - Final synthesized report

## Files

```
research_agent/
├── eunice.toml          # Multi-agent configuration
├── agents/
│   ├── lead.md          # Orchestrator prompt
│   ├── researcher.md    # Uses search_query with pro_preview
│   ├── report_writer.md # Synthesizes findings
│   └── evaluator.md     # Reviews report quality
├── research_notes/      # Output: research notes
└── reports/             # Output: final reports
```

## Design Notes

Following Anthropic's guidance on building effective agents:

1. **Orchestrator-workers** - Lead coordinates, researchers execute
2. **Evaluator-optimizer** - One evaluation/revision cycle for quality
3. **Clear tool docs** - Researcher always uses `pro_preview` model
4. **Bounded loops** - Only one revision to prevent infinite loops
