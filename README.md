# eunice

[![Crates.io](https://img.shields.io/crates/v/eunice.svg)](https://crates.io/crates/eunice)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

An agentic CLI runner in Rust with unified support for OpenAI, Gemini, Claude, and Ollama via OpenAI-compatible APIs.

**1,874 lines of Rust** - Emphasizing "sophisticated simplicity".

**Homepage**: [longrunningagents.com](https://longrunningagents.com)

## Features

- **Multi-Provider Support**: OpenAI, Google Gemini, Anthropic Claude, and local Ollama models
- **Unified API**: Uses OpenAI-compatible endpoints for all providers
- **MCP Integration**: Model Context Protocol servers for extensible tool capabilities
- **Smart Defaults**: Automatically selects the best available model
- **DMN Mode**: Default Mode Network - autonomous batch execution with pre-configured MCP tools for software engineering
- **Interactive Mode**: Multi-turn conversations with context preservation

## Installation

### From crates.io

```bash
cargo install eunice
```

### From Source

```bash
cargo build --release
```

The binary will be at `target/release/eunice`.

### Install Globally (from source)

```bash
cargo install --path .
```

## Quick Start

```bash
# Use smart default model (prefers local Ollama)
eunice "What files are in this directory?"

# Specify a model
eunice --model gemini-2.5-flash "Explain this code"
eunice --model sonnet "Review this implementation"
eunice --model llama3.1:latest "Summarize this text"

# Interactive mode
eunice --interact

# With MCP tools
eunice --config ./mcp-config.json "What time is it?"

# DMN mode - autonomous batch execution (auto-loads 7 MCP servers)
eunice --dmn "Fix the bug in main.rs"
```

## Command Line Options

```
Usage: eunice [OPTIONS] [PROMPT]

Arguments:
  [PROMPT]  Prompt as positional argument (can be file path or string)

Options:
      --model <MODEL>           AI model to use
      --prompt <PROMPT>         Prompt as file path or string
      --tool-output-limit <N>   Limit tool output display (default: 50, 0=unlimited)
      --list-models             Show all available models
      --config <FILE>           Path to MCP configuration JSON
      --no-mcp                  Disable MCP even if eunice.json exists
      --default-mode-network    Enable DMN mode with auto-loaded MCP tools [aliases: --dmn]
  -i, --interact                Interactive mode for multi-turn conversations
      --silent                  Suppress all output except AI responses
      --verbose                 Enable verbose debug output
      --events                  Output JSON-RPC events to stdout
  -h, --help                    Print help
  -V, --version                 Print version
```

## Provider Support

### OpenAI
- Models: `gpt-4o`, `gpt-4-turbo`, `gpt-4`, `gpt-3.5-turbo`, `o1`, `o1-mini`
- Requires: `OPENAI_API_KEY`

### Google Gemini
- Models: `gemini-2.5-flash`, `gemini-2.5-pro`, `gemini-1.5-flash`, `gemini-1.5-pro`
- Requires: `GEMINI_API_KEY`

### Anthropic Claude
- Models: `sonnet`, `opus`, `haiku` (aliases for latest versions)
- Full names: `claude-sonnet-4-20250514`, `claude-opus-4-1-20250805`, etc.
- Requires: `ANTHROPIC_API_KEY`

### Ollama (Local)
- Models: Any installed model (`llama3.1:latest`, `deepseek-r1:latest`, etc.)
- Requires: Ollama running at `http://localhost:11434` (or `OLLAMA_HOST`)

## Smart Model Selection

When no model is specified, eunice automatically selects the best available:

1. **Ollama** (if running): `gpt-oss:latest` → `deepseek-r1:latest` → `llama3.1:latest`
2. **Gemini** (if API key set): `gemini-2.5-flash`
3. **Anthropic** (if API key set): `sonnet`
4. **OpenAI** (if API key set): `gpt-4o`

## MCP Configuration

Create a `eunice.json` in your working directory for automatic MCP server loading:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    },
    "time": {
      "command": "uvx",
      "args": ["mcp-server-time"]
    }
  }
}
```

Tools are registered with server name prefixes (e.g., `filesystem_read_file`, `time_get_current_time`).

## DMN Mode (Default Mode Network)

Enable with `--dmn` (or `--default-mode-network`) for autonomous batch execution with 7 pre-configured MCP servers:

- **shell**: Execute shell commands
- **filesystem**: File operations
- **text-editor**: Line-based editing with conflict detection
- **grep**: Fast code search via ripgrep
- **memory**: Persistent memory storage
- **web**: Web search (requires `BRAVE_API_KEY`)
- **fetch**: HTTP requests

DMN mode executes tasks autonomously without stopping for confirmation. It makes reasonable decisions and proceeds through all steps automatically, following comprehensive system instructions for software engineering best practices.

## Project Structure

```
├── Cargo.toml           # Package configuration
├── Makefile             # Build commands
├── src/
│   ├── main.rs          # Entry point, CLI parsing (256 lines)
│   ├── config.rs        # Configuration + DMN instructions (274 lines)
│   ├── models.rs        # Data structures (223 lines)
│   ├── provider.rs      # Provider detection & routing (221 lines)
│   ├── display.rs       # Terminal UI (196 lines)
│   ├── client.rs        # HTTP client (108 lines)
│   ├── agent.rs         # Agent loop (107 lines)
│   ├── interactive.rs   # Interactive mode (120 lines)
│   └── mcp/
│       ├── server.rs    # MCP subprocess management (251 lines)
│       └── manager.rs   # Tool routing (139 lines)
└── README.md
```

## Dependencies

- **tokio**: Async runtime
- **reqwest**: HTTP client
- **clap**: CLI argument parsing
- **serde/serde_json**: Serialization
- **colored**: Terminal colors
- **anyhow/thiserror**: Error handling

## Examples

### Basic Usage

```bash
# Simple prompt
eunice "What is 2+2?"

# Read from file
eunice --prompt ./question.txt

# Silent mode (only AI response)
eunice --silent "Summarize this in one sentence"
```

### With MCP Tools

```bash
# Use time server
eunice --config ./config.json "What time is it in Tokyo?"

# The AI will call time_get_current_time or time_convert_time
```

### Interactive Session

```bash
$ eunice --interact --model sonnet
┌──────────────────────────────────────────────────┐
│ Model Info                                       │
├──────────────────────────────────────────────────┤
│ 🧠 Model: claude-sonnet-4-20250514 (Anthropic) │
└──────────────────────────────────────────────────┘

> What files are in this directory?
[Response with tool calls...]

> Now explain the main.rs file
[Continues with conversation context...]

> exit
```

### DMN Mode

```bash
# Complex software engineering task - runs autonomously
eunice --dmn "Find all TODO comments and create issues for them"

# The AI has access to shell, filesystem, grep, and more
# Executes all steps without stopping for confirmation
```

## Environment Variables

```bash
# API Keys
export OPENAI_API_KEY="sk-..."
export GEMINI_API_KEY="..."
export ANTHROPIC_API_KEY="..."
export BRAVE_API_KEY="..."  # For web search in DMN mode

# Ollama host (optional)
export OLLAMA_HOST="http://localhost:11434"
```

## License

MIT
