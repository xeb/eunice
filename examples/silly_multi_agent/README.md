# Silly Multi-Agent Example

A demonstration of multi-agent orchestration using eunice, where a root agent coordinates two specialized sub-agents.

## Architecture

```
silly_multi_agent/
├── root_agent/           # The orchestrator
│   ├── eunice.toml       # shell + filesystem MCP
│   ├── instructions.md   # Coordination logic
│   └── workspace/        # Orchestration logs & final summary
│
├── researcher_agent/     # Web research specialist
│   ├── eunice.toml       # brave search + filesystem MCP
│   ├── instructions.md   # Research instructions
│   └── workspace/        # Research findings
│
└── analyzer_agent/       # Code analysis specialist
    ├── eunice.toml       # shell + filesystem MCP
    ├── instructions.md   # Analysis instructions
    └── workspace/        # Cloned repos & analyses
```

## How It Works

1. **researcher_agent** uses Brave Search to find LLM agent frameworks
2. **analyzer_agent** clones and analyzes the found repositories
3. **root_agent** orchestrates both by invoking them via shell commands

The root agent calls sub-agents like this:
```bash
cd ../researcher_agent && eunice --prompt="Research LLM agent frameworks..."
cd ../analyzer_agent && eunice --prompt="Analyze projects found by researcher..."
```

## Prerequisites

- eunice installed (`cargo install eunice`)
- mcpz installed (`cargo install mcpz`)
- `BRAVE_API_KEY` environment variable set (for researcher_agent)

## Running

### Option 1: Run the Root Orchestrator
```bash
cd root_agent
eunice --prompt="Please orchestrate a research session. Run the researcher first, then the analyzer, then compile a summary."
```

### Option 2: Run Agents Individually
```bash
# Step 1: Research
cd researcher_agent
eunice --prompt="Find interesting LLM agent frameworks"

# Step 2: Analyze
cd analyzer_agent
eunice --prompt="Analyze projects from ../researcher_agent/workspace/"

# Step 3: Summarize (manual or via root_agent)
```

### Option 3: Use the run script
```bash
./run.sh
```

## Output

After a successful run, you'll have:

- `researcher_agent/workspace/research_log.md` - Discovered projects
- `researcher_agent/workspace/projects_to_analyze.md` - URLs for analyzer
- `analyzer_agent/workspace/repos/` - Cloned repositories
- `analyzer_agent/workspace/analyses/` - Detailed analysis files
- `root_agent/workspace/final_summary.md` - Compiled insights

## Why "Silly"?

This is intentionally simple and somewhat naive - it demonstrates the concept of multi-agent coordination without fancy frameworks. Just shell commands and eunice.

The agents communicate via the filesystem (shared workspace files), which is crude but effective for learning purposes.

## Known Issues

The `@modelcontextprotocol/server-filesystem` MCP server has a bug (`keyValidator._parse is not a function`) that causes filesystem tool calls to fail. However, the agents are smart enough to work around this by falling back to shell commands (`cat`, `mkdir`, heredocs, etc.). This demonstrates agent resilience!
