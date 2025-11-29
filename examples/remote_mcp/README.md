# Remote MCP Example

Demonstrates connecting to a remote MCP server via Streamable HTTP transport.

## Setup

Start a remote MCP server on port 3323:

```bash
mcpz server shell --http 3323
```

## Usage

```bash
cd examples/remote_mcp
eunice "list files in the current directory"
```

The agent will use the remote shell server to execute commands and provide suggestions.
