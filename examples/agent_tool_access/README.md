# Agent Tool Access Patterns

This example demonstrates the three ways to control which MCP tools an agent can access.

## Configuration Naming

Both camelCase and snake_case are accepted:
- `mcpServers` or `mcp_servers` - for server definitions
- `allowedTools` or `allowed_tools` - for global filtering
- `deniedTools` or `denied_tools` - for global blacklisting

## Tool Access Methods

### 1. Server-Level Access (`mcp_servers` only)

Agent gets **ALL tools** from the specified servers:

```toml
[agents.shell_expert]
prompt = "You are a shell expert..."
mcp_servers = ["shell"]      # Gets ALL tools from the shell server
```

### 2. Pattern-Based Access (`tools` only)

Agent gets tools matching patterns from **ALL servers**:

```toml
[agents.reader]
prompt = "You are a read-only agent..."
tools = ["*_read*", "*_list*"]    # Gets matching tools from any server
```

### 3. Combined Access (`mcp_servers` + `tools`)

Agent gets **filtered tools** from **specific servers**:

```toml
[agents.fs_writer]
prompt = "You are a filesystem writer..."
mcp_servers = ["filesystem"]       # Only look at this server
tools = ["*_write*", "*_edit*"]    # Only get these patterns
```

## Pattern Syntax

Tool patterns support wildcards:
- `shell_execute` - exact match
- `shell_*` - prefix match (all shell tools)
- `*_read` - suffix match (all read tools)
- `*_file_*` - contains match

## Usage

```bash
# See all agents and their tool access
eunice --list-agents

# See all available tools
eunice --list-tools

# Run with root coordinator
eunice "list the files in the current directory"
```
