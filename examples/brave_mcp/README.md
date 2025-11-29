# Brave Web Search Example

This example demonstrates using the Brave Search MCP server for web searches without DMN mode.

## Prerequisites

1. Get a Brave Search API key from https://brave.com/search/api/
2. Set the environment variable:
   ```bash
   export BRAVE_API_KEY="your-api-key-here"
   ```

## Usage

```bash
cd examples/brave_mcp

# Simple web search
eunice "What is the latest version of Rust?"

# Search for documentation
eunice "Find the official MCP protocol documentation"

# Research a topic
eunice "What are the best practices for Rust error handling in 2024?"
```

## How It Works

The `eunice.toml` configures a single MCP server:

```toml
[mcpServers.web]
command = "npx"
args = ["-y", "@anthropic-ai/mcp-server-brave-search"]
```

This provides the following tools:
- `web_brave_web_search` - General web search
- `web_brave_local_search` - Local business search
- `web_brave_news_search` - News search

## Available Tools

When you run `eunice` in this directory, the model has access to:

| Tool | Description |
|------|-------------|
| `web_brave_web_search` | Search the web for information |
| `web_brave_local_search` | Search for local businesses |
| `web_brave_news_search` | Search news articles |

## Notes

- This is NOT DMN mode - just a single MCP server configuration
- The API key must be set before running
- Results are returned as structured data from Brave Search API
