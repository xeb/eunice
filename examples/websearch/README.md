# Web Search Example

This example demonstrates using Eunice's built-in `search_query` tool for web searches with Google Search grounding via Gemini models.

## Files

- `instructions.md` - Prompt asking to search for current information
- `run.sh` - Script to run the example

## Usage

### With DMN Mode (includes all MCP tools + search)

```bash
cd examples/websearch
eunice --dmn --prompt instructions.md
```

### With --search Flag (standalone, no MCP)

```bash
cd examples/websearch
eunice --search --no-mcp "What are the latest developments in AI?"
```

### Using run.sh

```bash
./run.sh
```

## How It Works

1. The model receives a prompt requiring current information
2. It calls the `search_query` tool with:
   - `query`: The search query
   - `model`: One of `flash`, `pro`, or `pro_preview`
3. Eunice makes an API request to the selected Gemini model with Google Search grounding
4. The AI-synthesized search results are returned, including source citations

## Model Selection Guide

- **flash** (gemini-2.5-flash): Quick knowledge queries, fast and cheap
- **pro** (gemini-2.5-pro): Medium complexity queries requiring deeper analysis
- **pro_preview** (gemini-3-pro-preview): Hardest queries requiring maximum reasoning

## Requirements

- Eunice installed (`cargo install eunice`)
- `GEMINI_API_KEY` environment variable set
