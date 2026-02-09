# Web Search Skill

## Description
Search the web for current information using DuckDuckGo (no API key required) or Gemini with Google Search grounding (requires GEMINI_API_KEY).

## Scripts

### search.py
A Python script for web search. Run with `uv run`:

```bash
# DuckDuckGo search (no API key needed)
uv run ~/.eunice/skills/web_search/search.py "python async tutorial"

# Gemini search with Google grounding (requires GEMINI_API_KEY)
uv run ~/.eunice/skills/web_search/search.py "Who won Euro 2024?" --gemini

# Limit DuckDuckGo results
uv run ~/.eunice/skills/web_search/search.py "rust error handling" --max 3

# Search and fetch first result content
uv run ~/.eunice/skills/web_search/search.py "Node.js latest version" --fetch

# Search news
uv run ~/.eunice/skills/web_search/search.py "AI announcements" --news

# Output as JSON
uv run ~/.eunice/skills/web_search/search.py "query" --json
```

### Requirements
- `uv` (Python package manager)
- Internet connection
- For Gemini search: `GEMINI_API_KEY` environment variable

## Examples

### Quick Factual Questions (Gemini)
```bash
uv run ~/.eunice/skills/web_search/search.py "What is the current price of Bitcoin?" --gemini
uv run ~/.eunice/skills/web_search/search.py "Who is the current US president?" --gemini
uv run ~/.eunice/skills/web_search/search.py "When is the next solar eclipse?" --gemini
```

### Programming Questions (DuckDuckGo + Fetch)
```bash
uv run ~/.eunice/skills/web_search/search.py "Python dataclass tutorial" --fetch
uv run ~/.eunice/skills/web_search/search.py "Rust async await guide" --fetch
uv run ~/.eunice/skills/web_search/search.py "React hooks best practices" --max 3
```

### Current Events (Gemini)
```bash
uv run ~/.eunice/skills/web_search/search.py "latest news about AI regulation" --gemini
uv run ~/.eunice/skills/web_search/search.py "recent tech layoffs 2024" --gemini
```

### News Search (DuckDuckGo)
```bash
uv run ~/.eunice/skills/web_search/search.py "OpenAI announcements" --news
uv run ~/.eunice/skills/web_search/search.py "climate change" --news --max 10
```

### Research with Sources (Gemini)
```bash
uv run ~/.eunice/skills/web_search/search.py "health benefits of intermittent fasting" --gemini
uv run ~/.eunice/skills/web_search/search.py "compare PostgreSQL vs MySQL performance" --gemini
```

### Get Raw Links (DuckDuckGo JSON)
```bash
uv run ~/.eunice/skills/web_search/search.py "rust programming tutorials" --json | jq '.results[].url'
```

### Different Gemini Models
```bash
# Fast responses
uv run ~/.eunice/skills/web_search/search.py "weather in Tokyo" --gemini --model gemini-2.0-flash

# More detailed analysis
uv run ~/.eunice/skills/web_search/search.py "explain quantum computing advances" --gemini --model gemini-2.5-pro
```

## When to Use Which

| Use Case | Recommended |
|----------|-------------|
| Quick factual questions | `--gemini` |
| Programming docs/tutorials | DuckDuckGo + `--fetch` |
| Current events | `--gemini` or `--news` |
| Get list of URLs | DuckDuckGo |
| Detailed research with sources | `--gemini` |
| No API key available | DuckDuckGo (default) |
