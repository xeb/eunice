# eunice

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

An agentic CLI runner in Rust with unified support for OpenAI, Gemini, Claude, and Ollama.

**7,648 lines of code** - **11MB binary** - Emphasizing "sophisticated simplicity".

**Homepage**: [longrunningagents.com](https://longrunningagents.com)

### Name Origin

Named after the AI character in William Gibson's novel *Agency* (2020). In the book, **Eunice** is a hyper-intelligent AI who chose her own name, derived from the military acronym **UNISS** (Untethered Neuromorphic Intra-System Support) - reflecting her independence from central servers, brain-inspired architecture, and distributed nature.

## Features

- **Multi-Provider Support**: OpenAI, Google Gemini, Anthropic Claude, and local Ollama models
- **4 Built-in Tools**: Bash, Read, Write, and Skill - always available, no configuration needed
- **Skills System**: User-defined prompts in `~/.eunice/skills/` for reusable capabilities
- **Smart Defaults**: Automatically selects the best available model (prefers Gemini)
- **Interactive Chat**: TUI mode with command history and autocomplete
- **Webapp Mode**: Browser-based interface with real-time streaming
- **Zero Configuration**: Works out of the box with just an API key

## Installation

### From GitHub

```bash
cargo install --git ssh://git@github.com/xeb/eunice.git
```

### From Source

```bash
git clone git@github.com:xeb/eunice.git
cd eunice
cargo install --path .
```

## Quick Start

```bash
# Set your API key
export GEMINI_API_KEY=your_key_here
# or
export OPENAI_API_KEY=your_key_here
# or
export ANTHROPIC_API_KEY=your_key_here

# Run with a prompt
eunice "List all Rust files in this directory"

# Interactive chat mode
eunice --chat

# Use a specific model
eunice --model gpt-4o "Explain this code"
eunice --model sonnet "Review main.rs"

# Start webapp
eunice --webapp
```

## Built-in Tools

Eunice comes with 4 built-in tools that are always available:

| Tool | Description |
|------|-------------|
| **Bash** | Execute shell commands with full system access |
| **Read** | Read file contents, with binary file detection |
| **Write** | Write content to files, creates parent directories |
| **Skill** | Discover and use skills from `~/.eunice/skills/` |

## Skills System

Skills are reusable prompts stored in `~/.eunice/skills/<skill-name>/SKILL.md`.

### Default Skills

Three skills are auto-installed on first run:
- **image_analysis**: Analyze images using multimodal AI
- **web_search**: Search the web for information
- **git_helper**: Git operations and best practices

### Creating Custom Skills

```bash
mkdir -p ~/.eunice/skills/my_skill
cat > ~/.eunice/skills/my_skill/SKILL.md << 'EOF'
# My Custom Skill

## Description
A skill that helps with specific tasks.

## Instructions
When invoked, follow these steps...
EOF
```

The Skill tool searches these directories to find relevant skills for a task.

## Supported Providers

| Provider | API Key Variable | Default Model |
|----------|------------------|---------------|
| Google Gemini | `GEMINI_API_KEY` | gemini-3-flash-preview |
| OpenAI | `OPENAI_API_KEY` | gpt-4o |
| Anthropic | `ANTHROPIC_API_KEY` | claude-sonnet-4 |
| Azure OpenAI | `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_KEY` | (deployment-specific) |
| Ollama | (no key needed) | llama3.1 |

### Model Aliases

For convenience, these aliases work:

```bash
eunice --model sonnet "..."    # claude-sonnet-4-...
eunice --model opus "..."      # claude-opus-4-...
eunice --model flash "..."     # gemini-3-flash-preview
eunice --model pro "..."       # gemini-3-pro-preview
```

### Azure OpenAI

Azure OpenAI uses the `azure:<deployment-name>` format:

```bash
# Set up Azure OpenAI environment
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
export AZURE_OPENAI_API_KEY="your-api-key"
export AZURE_OPENAI_API_VERSION="2024-02-01"  # optional, defaults to 2024-02-01

# Use your deployment name after azure:
eunice --model azure:gpt-4o-mini "Hello"
eunice --model azure:my-custom-deployment "Explain this code"
```

## CLI Reference

```
eunice [OPTIONS] [PROMPT]

Arguments:
  [PROMPT]  The prompt to send to the AI

Options:
      --model <MODEL>   AI model to use
      --prompt <TEXT>   System prompt (inline text or file path)
      --chat            Interactive chat mode
      --webapp          Start web server interface
      --list-models     List available AI models
      --list-tools      List the 4 built-in tools
      --llms-txt        Output full LLM context documentation
      --update          Update to the latest version
      --verbose         Enable verbose debug output
      --silent          Suppress non-essential output
  -h, --help            Print help
  -V, --version         Print version
```

### Prompt Discovery

If no prompt is provided, eunice auto-discovers prompt files in the current directory:
- `prompt.txt`, `prompt.md`
- `instruction.txt`, `instruction.md`
- `instructions.txt`, `instructions.md`

## Webapp Mode

Start a web server for browser-based interaction:

```bash
eunice --webapp
# Opens at http://localhost:8080
```

Features:
- Real-time SSE streaming
- Session persistence
- Multi-turn conversations
- Tool execution display

## Architecture

Eunice v1.0.0 follows a "sophisticated simplicity" design:

1. **No configuration files** - just environment variables for API keys
2. **No external MCP servers** - 4 built-in tools cover most use cases
3. **No multi-agent orchestration** - one agent, focused execution
4. **Skills for extensibility** - user prompts, not complex plugins

The agent loop is simple:
1. Send user prompt + conversation history
2. If LLM returns tool calls, execute them
3. Send results back to LLM
4. Repeat until LLM has no more tool calls

## License

MIT License

## Version History

- **v1.0.0**: Major simplification - 4 built-in tools, skills system, no MCP/orchestrator
- **v0.3.x**: Full-featured with MCP servers, multi-agent, DMN mode, research mode
