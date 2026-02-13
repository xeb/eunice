# Web Search Example

This example demonstrates using Eunice's web_search skill for searching the web.

## Files

- `instructions.md` - Prompt asking to search for current information
- `run.sh` - Script to run the example

## Usage

```bash
cd examples/websearch
./run.sh
```

Or directly:

```bash
eunice --prompt instructions.md "Search for AI news"
```

## How It Works

1. The agent receives a prompt requiring current information
2. It uses the Skill tool to find the web_search skill
3. It runs the search.py script via Bash to perform the search
4. Results are returned with source citations

## Requirements

- Eunice installed
- `uv` (Python package manager)
- For Gemini search with grounding: `GEMINI_API_KEY` environment variable
- DuckDuckGo search works without any API key
