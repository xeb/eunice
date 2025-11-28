# Multi-Agent Architecture Design for Eunice

## Overview

This document proposes a multi-agent architecture where agents are first-class citizens that can communicate via tool calls. Each agent is defined by a system prompt and a set of available MCP tools. Agents can invoke other agents as if they were tools, enabling hierarchical and collaborative workflows.

## Core Concepts

### Agent Definition

An agent consists of:
- **name**: Unique identifier (e.g., `root`, `senior_dev`, `tester`)
- **prompt**: System instructions defining the agent's role and behavior
- **tools**: List of MCP tools this agent can access
- **can_invoke**: List of other agent names this agent can call

### Agent as Tool

When Agent A can invoke Agent B, Agent B appears as a tool to Agent A:
```
Tool: invoke_agent
Parameters:
  - agent: "senior_dev"
  - task: "Implement the authentication module"
  - context: { ... optional shared context ... }
Returns:
  - result: Agent B's final response
  - artifacts: Any files created/modified
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      AgentOrchestrator                       │
│  - Manages agent registry                                    │
│  - Routes inter-agent calls                                  │
│  - Maintains shared context/memory                           │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
   ┌─────────┐          ┌─────────┐          ┌─────────┐
   │  Agent  │          │  Agent  │          │  Agent  │
   │  Loop   │          │  Loop   │          │  Loop   │
   └────┬────┘          └────┬────┘          └────┬────┘
        │                    │                    │
   ┌────┴────┐          ┌────┴────┐          ┌────┴────┐
   │   MCP   │          │   MCP   │          │   MCP   │
   │  Tools  │          │  Tools  │          │  Tools  │
   └─────────┘          └─────────┘          └─────────┘
```

## Configuration Format

Extend `eunice.toml` to support agent definitions:

```toml
# MCP servers (existing format) - agents reference these by name
[mcpServers.shell]
command = "mcpz"
args = ["server", "shell"]

[mcpServers.filesystem]
command = "mcpz"
args = ["server", "filesystem"]

[mcpServers.text-editor]
command = "uvx"
args = ["mcp-text-editor"]

[mcpServers.grep]
command = "npx"
args = ["-y", "mcp-ripgrep@latest"]

[mcpServers.memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]

[mcpServers.web]
command = "npx"
args = ["-y", "@anthropic/brave-search-mcp-server"]

# Agent definitions
[agents.root]
prompt = "You are the CEO. Delegate tasks to appropriate agents."
can_invoke = ["product_manager", "senior_dev", "marketing"]
# root has no direct tools - only delegates

[agents.product_manager]
prompt = "agents/product_manager.md"  # Can be a file path
tools = ["filesystem", "memory"]       # References mcpServers by name
can_invoke = ["senior_dev", "tester"]

[agents.senior_dev]
prompt = "agents/senior_dev.md"
tools = ["shell", "filesystem", "text-editor", "grep"]
can_invoke = ["junior_dev", "tester"]

[agents.junior_dev]
prompt = "You implement specific tasks assigned by senior dev. Focus on clean, working code."
tools = ["shell", "filesystem", "text-editor"]
can_invoke = []  # Leaf agent - cannot delegate

[agents.tester]
prompt = "agents/tester.md"
tools = ["shell", "filesystem", "grep"]
can_invoke = []

[agents.marketing]
prompt = "You create marketing content and strategies."
tools = ["filesystem", "web"]
can_invoke = []
```

### Configuration Details

**`prompt`** - Can be either:
- Inline string: `prompt = "You are a developer..."`
- File path: `prompt = "agents/senior_dev.md"` (relative to config file)

**`tools`** - Array of MCP server names from `[mcpServers.*]`:
- `tools = ["shell", "filesystem"]` gives access to all tools from those servers
- Empty or omitted means no MCP tools (agent can only delegate or respond)

**`can_invoke`** - Array of agent names this agent can call:
- Creates `invoke_<agent_name>` tools automatically
- Empty array means leaf agent (cannot delegate)

## Implementation Approach

### Option A: Internal MCP Server (Recommended)

Register an internal MCP server called `agents` that exposes each agent as a tool:

```rust
// Pseudo-code
struct AgentMcpServer {
    orchestrator: Arc<AgentOrchestrator>,
}

impl AgentMcpServer {
    fn list_tools(&self) -> Vec<Tool> {
        self.orchestrator.agents.iter().map(|agent| {
            Tool {
                name: format!("agents_invoke_{}", agent.name),
                description: format!("Invoke the {} agent", agent.name),
                parameters: json!({
                    "task": { "type": "string", "description": "Task to assign" },
                    "context": { "type": "object", "description": "Shared context" }
                })
            }
        }).collect()
    }

    async fn call_tool(&self, name: &str, params: Value) -> Result<Value> {
        let agent_name = name.strip_prefix("agents_invoke_").unwrap();
        let task = params["task"].as_str().unwrap();
        let context = params.get("context");

        self.orchestrator.invoke_agent(agent_name, task, context).await
    }
}
```

**Pros:**
- Agents appear as normal tools - no special handling in agent loop
- Reuses existing MCP infrastructure
- Clean separation of concerns

**Cons:**
- Synchronous by default (calling agent blocks until complete)

### Option B: Message Queue Architecture

Agents communicate via an async message queue:

```rust
struct AgentMessage {
    from: String,
    to: String,
    task: String,
    context: Value,
    reply_to: Option<oneshot::Sender<AgentResponse>>,
}

struct AgentOrchestrator {
    tx: mpsc::Sender<AgentMessage>,
    agents: HashMap<String, AgentHandle>,
}
```

**Pros:**
- True async - agents can work in parallel
- Supports fire-and-forget patterns
- More scalable for complex workflows

**Cons:**
- More complex implementation
- Harder to debug
- Need to handle message ordering

### Option C: Hybrid (Recommended for v1)

Start with synchronous internal MCP server, add async capabilities later:

1. **Phase 1**: Sync agent invocation via internal MCP
2. **Phase 2**: Add `invoke_agent_async` tool that returns a task ID
3. **Phase 3**: Add `check_agent_task` and `wait_agent_task` tools

## Changes Required in Eunice

### New Files

```
src/
├── orchestrator/
│   ├── mod.rs           # Module exports
│   ├── agent.rs         # Agent struct and loop
│   ├── orchestrator.rs  # AgentOrchestrator
│   ├── config.rs        # Agent config parsing
│   └── mcp_bridge.rs    # Internal MCP server for agents
```

### Modified Files

1. **src/config.rs**
   - Parse `[agents.*]` sections from config
   - Load agent prompts (inline or from files)

2. **src/mcp/manager.rs**
   - Register internal agent MCP server alongside external servers
   - Route `agents_*` tool calls to orchestrator

3. **src/main.rs**
   - New `--agent <name>` flag to start as specific agent
   - New `--multi-agent` flag to enable orchestrator mode

4. **src/models.rs**
   - Add `AgentConfig` struct
   - Add `AgentMessage` for inter-agent communication

### Minimal Implementation (~200 lines)

```rust
// src/orchestrator/agent.rs
pub struct AgentConfig {
    pub name: String,
    pub prompt: String,
    pub tools: Vec<String>,
    pub can_invoke: Vec<String>,
}

pub struct AgentOrchestrator {
    agents: HashMap<String, AgentConfig>,
    client: Client,
    mcp_manager: McpManager,
}

impl AgentOrchestrator {
    pub async fn invoke_agent(
        &self,
        name: &str,
        task: &str,
        context: Option<&Value>
    ) -> Result<String> {
        let agent = self.agents.get(name)
            .ok_or_else(|| anyhow!("Unknown agent: {}", name))?;

        // Build prompt with task and context
        let full_prompt = format!(
            "{}\n\n# TASK\n{}\n\n# CONTEXT\n{}",
            agent.prompt,
            task,
            context.map(|c| c.to_string()).unwrap_or_default()
        );

        // Run agent loop with filtered tools
        let mut history = Vec::new();
        run_agent(
            &self.client,
            &self.model,
            &full_prompt,
            50,
            Some(&mut self.get_filtered_mcp(agent)),
            false,
            false,
            &mut history,
            false,
        ).await
    }

    fn get_filtered_mcp(&self, agent: &AgentConfig) -> McpManager {
        // Return MCP manager with only tools this agent can access
        // Plus invoke_* tools for agents in can_invoke list
    }
}
```

## Example Workflow

1. User runs: `eunice --multi-agent "Build a todo app with tests"`

2. RootAgent receives task, decides to delegate:
   ```
   I'll coordinate this project.

   First, let me get requirements from ProductManager.
   [calls agents_invoke_product_manager]
   ```

3. ProductManager creates spec, returns to RootAgent

4. RootAgent delegates implementation:
   ```
   Now I'll have SeniorDev implement this.
   [calls agents_invoke_senior_dev with spec as context]
   ```

5. SeniorDev breaks down work, delegates to JuniorDev:
   ```
   I'll architect this and have JuniorDev implement the UI.
   [calls agents_invoke_junior_dev]
   ```

6. SeniorDev requests testing:
   ```
   [calls agents_invoke_tester]
   ```

7. Results bubble back up to RootAgent

## Shared Memory / Context

Agents need shared state. Options:

1. **Pass context in tool calls** - Simple but verbose
2. **Shared memory MCP server** - Already have this in DMN mode
3. **Orchestrator-managed context** - Central store accessible to all agents

Recommendation: Use existing `memory` MCP server, all agents get access to it.

## CLI Interface

Only one flag needed: `--agent <name>`. Multi-agent mode is auto-detected when `[agents]` section exists in config.

```bash
# Auto-detects agents in eunice.toml, starts with "root" agent by default
eunice "Build a todo app"

# Explicitly start as a specific agent
eunice --agent senior_dev "Implement auth module"

# Start as root (explicit, same as default when agents configured)
eunice --agent root "Build a todo app"

# Run single agent without multi-agent orchestration (current behavior)
# Just don't define [agents] section, or use --no-agents
eunice --no-agents "Simple question"

# List configured agents
eunice --list-agents

# Interactive mode with agents
eunice --agent root -i
```

**Behavior:**
- If `[agents]` section exists and no `--agent` specified → use `root` agent
- If `[agents]` section exists and `--agent X` specified → use agent X
- If no `[agents]` section → current single-agent behavior (backward compatible)

## Future Enhancements

1. **Async agent pools** - Multiple junior_dev instances
2. **Agent memory** - Per-agent persistent context
3. **Approval workflows** - Human-in-the-loop for critical decisions
4. **Agent metrics** - Token usage, task completion rates
5. **Visual workflow** - Graph of agent interactions
6. **Agent templates** - Pre-built agent configurations

## Summary

The recommended approach is **Option C (Hybrid)**:

1. Implement agents as an internal MCP server
2. Agent-to-agent calls are synchronous tool calls
3. Shared context via memory MCP server
4. Configuration in `eunice.toml` under `[agents.*]`
5. ~200-300 lines of new code for core functionality

This keeps the implementation simple while enabling powerful multi-agent workflows. The existing MCP infrastructure handles tool routing, and agents appear as just another set of tools to the agent loop.
