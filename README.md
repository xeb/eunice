# eunice

A generalist, minimalist agent framework for natural language interactions with AI models that can execute local file operations.

## Installation

```bash
uv tool install .
```

## Usage

### Basic Commands
```bash
# Ask questions about your files
eunice "How many files are in the current directory?"

# Use different models
eunice --model="gpt-4" "analyze this codebase"
eunice --model="gemini-2.5-pro" "what does the main file do?"
eunice --model="llama3.1" "summarize the project structure"

# List available models
eunice --list-models

# Use file prompts
eunice --prompt=analysis_request.txt
```

### Configuration

Set API keys as environment variables:
```bash
export OPENAI_API_KEY="your-openai-key"
export GEMINI_API_KEY="your-gemini-key"
# Ollama models run locally (no API key needed)
```

### Options

- `--model=MODEL` - Choose AI model (default: gpt-3.5-turbo)
- `--prompt=PROMPT` - Prompt as file or string
- `--tool-output-limit=N` - Limit tool output display (default: 50)
- `--list-models` - Show available models
- `--help` - Show help with API key status

## Supported Models

- **OpenAI**: gpt-3.5-turbo, gpt-4, gpt-4o, gpt-5, etc.
- **Gemini**: gemini-2.5-flash, gemini-2.5-pro, etc.
- **Ollama**: Any locally installed model

## Development

```bash
# Run directly with uv
uv run eunice.py "your prompt"

# Run tests
./test.sh

# Uninstall
uv tool uninstall eunice
```

For detailed documentation, see [CLAUDE.md](CLAUDE.md).