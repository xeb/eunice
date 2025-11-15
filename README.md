# eunice

An agent tool and framework that runs with different LLM providers (including Ollama), supports MCP tools (stdio only for now), runs in a loop, has a docker image (for isolation), and support an interactive chat mode as needed.

> **Name Origin**: eunice is named after the AI character Eunice from William Gibson's novel "Agency" - a highly capable artificial intelligence that assists with complex tasks through natural conversation.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│   📝 Prompt    +    🔧 MCP Config    +    🤖 eunice                        │
│                                                                             │
│                            ║                                                │
│                            ║                                                │
│                            ▼                                                │
│                                                                             │
│              ✨ Model-Agnostic Agentic Runtime ✨                          │
│                                                                             │
│        (OpenAI • Gemini • Anthropic • Ollama)                              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Sample Usage

![eunice in action](assets/session.gif)

Here's a comprehensive example showing eunice in action with MCP server configuration:

```bash
time uvx git+https://github.com/xeb/eunice --verbose --model=gemini-2.5-pro --config=my_mcp_servers.json prompt.txt
```

**Sample MCP Configuration** (`my_mcp_servers.json`):
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
    },
    "fetch": {
      "command": "uvx",
      "args": ["mcp-server-fetch"]
    },
    "memory": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory", "."]
    }
  }
}
```

**Sample Prompt File** (`prompt.txt`):
```
Analyze this project directory and help me understand:

1. What is the current time and date?
2. What files and directories exist in this project?
3. Read the main Python file and summarize its purpose
4. Store a brief project summary in memory for future reference
5. If there's a README, fetch any external links mentioned and summarize what they contain

Please be thorough and use the available tools to provide a comprehensive analysis.
```

This example demonstrates:
- Using `uvx` for one-time execution without installation
- Verbose mode (`--verbose`) for detailed debugging output to `/tmp/eunice_debug.log`
- Multiple MCP servers (filesystem, time, fetch, memory) working together
- File-based prompts for complex multi-step tasks
- Timing the execution with the `time` command

## Project Goals

**Minimalism**: Keep the core implementation under 2,000 lines of Python code.

✅ **Current Status**: `eunice.py` is **1,343/2,000 lines** (67.2% used, **32.8% remaining**)

## Installation

```bash
uv tool install git+https://github.com/xeb/eunice
```

Or if you want to clone this repo & install locally:

```bash
git clone https://github.com/xeb/eunice.git
uv tool install .
```

## Usage

### Basic Commands
```bash
# Ask questions about your files
eunice "How many files are in the current directory?"

# Use different models (smart defaults prioritize available models)
eunice "analyze this codebase"  # Uses smart default selection
eunice --model="gpt-4" "analyze this codebase"
eunice --model="gemini-2.5-pro" "what does the main file do?"
eunice --model="sonnet" "explain the code structure"
eunice --model="opus" "review this implementation"
eunice --model="llama3.1" "summarize the project structure"

# Interactive mode for ongoing conversations
eunice --interact

# List available models
eunice --list-models

# Use file prompts
eunice --prompt=analysis_request.txt

# Use MCP configuration for extended tool capabilities
eunice --config=mcp-config.json "What time is it and list the files?"

# Enable verbose debugging output
eunice --verbose "debug tool execution to /tmp/eunice_debug.log"

# Disable MCP even if eunice.json exists
eunice --no-mcp "analyze code without any MCP tools"
```

### Docker Usage
You can also use the official Docker image instead of installing eunice locally:

```bash
# List all available models (host Ollama + cloud models with API keys)
docker run -e OPENAI_API_KEY -e GEMINI_API_KEY -e ANTHROPIC_API_KEY --network host --rm -it -e OLLAMA_HOST=http://localhost:11434 xebxeb/eunice eunice --list-models

# List available models (uses host Ollama API)
docker run --rm --network host xebxeb/eunice eunice --list-models

# Run with Ollama models (connects to host Ollama web API)
docker run --rm --network host xebxeb/eunice eunice --model="gpt-oss:latest" "What is best in life?"

# Run with cloud models (pass API keys as environment variables)
docker run --rm -e OPENAI_API_KEY="$OPENAI_API_KEY" xebxeb/eunice eunice --model="gpt-4" "Hello world"
```

### Configuration

#### API Keys
Set API keys as environment variables:
```bash
export OPENAI_API_KEY="your-openai-key"
export GEMINI_API_KEY="your-gemini-key"
export ANTHROPIC_API_KEY="your-anthropic-key"
# Ollama models run locally (no API key needed)
```

#### MCP Configuration
eunice supports Model Context Protocol (MCP) servers for extended tool capabilities:

**Automatic Configuration**: If a file named `eunice.json` exists in the current directory, it will be automatically loaded as the MCP configuration.

**Manual Configuration**: Use `--config=path/to/config.json` to specify a custom configuration file.

**Disabling MCP**: Use `--no-mcp` to disable MCP server loading even if `eunice.json` exists. You can also use `--config=''` (empty string) for the same effect.

**Example Configuration** (`eunice.json`):
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
    },
    "fetch": {
      "command": "uvx",
      "args": ["mcp-server-fetch"]
    }
  }
}
```

See `config.example.json` for a comprehensive configuration with all available MCP servers.

**Available MCP Servers**:
- **filesystem**: File operations (read, write, list directories)
- **time**: Time and date operations
- **fetch**: Web requests and API calls
- **memory**: Persistent storage and retrieval
- **sequential-thinking**: Step-by-step reasoning

**Usage Examples**:
```bash
# Automatic config loading (if eunice.json exists)
eunice "What time is it and how many files are here?"

# Manual config specification
eunice --config=tests/custom-config.json "Fetch data from an API"

# No config (basic file operations only)
eunice "Analyze this codebase structure"

# Explicitly disable MCP servers
eunice --no-mcp "Simple analysis without any tools"
```

### Options

- `--model=MODEL` - Choose AI model (smart default: available Ollama models → Gemini → Anthropic → OpenAI)
- `--prompt=PROMPT` - Prompt as file or string
- `--tool-output-limit=N` - Limit tool output display (default: 50)
- `--silent` - Suppress all output except AI responses (hide tool calls and model info)
- `--verbose` - Enable verbose debug output to /tmp/eunice_debug.log
- `--no-mcp` - Disable MCP server loading even if eunice.json exists
- `--list-models` - Show available models
- `--version` - Show program version number
- `--help` - Show help with API key status
- `--interact` - Start interactive mode for ongoing conversations

## Supported Models

- **OpenAI**: gpt-3.5-turbo, gpt-4, gpt-4o, gpt-5, etc.
- **Gemini**: gemini-2.5-flash, gemini-2.5-pro, etc.
- **Anthropic**: claude-sonnet-4-20250514, claude-opus-4-1-20250805, or use aliases: `sonnet`, `opus`, `claude-sonnet`, `claude-opus`
- **Ollama**: Any locally installed model

## Testing

eunice includes a comprehensive test suite to validate functionality:

```bash
# Run all tests locally (optimized for speed)
make test-host

# Run tests in Docker (clean environment)
make test-docker

# Run all available tests
make test

# Other useful commands
make help          # Show all available commands
make install       # Install eunice locally
make build-and-test # Build Docker image and run tests
```

### Test Coverage
- **26 comprehensive tests** covering all features
- Provider detection (OpenAI, Gemini, Anthropic, Ollama)
- Model validation and routing with smart defaults
- MCP server integration and tool functionality
- Interactive mode with proper display handling
- Silent mode and colored output
- Error handling and edge cases
- Command line argument parsing
- **60% faster execution** with optimized --no-mcp flags for tests that don't need MCP

### Docker Testing
The Docker test environment:
- Uses Alpine Linux for minimal footprint
- **Optimized layers** (reduced from 9 to 6 layers)
- Connects to host Ollama via port binding
- Validates clean installation process
- Tests all functionality in isolated environment

## Examples

eunice includes several practical examples demonstrating different use cases:

### 🤖 Multi-Agent Story Writing (`examples/multi_agent/`)
A sophisticated example showing how eunice can orchestrate multiple "agents" to collaboratively create content:
- **Writer agent**: Creates initial cyberpunk stories
- **Editor agent**: Improves story pacing and character development
- **Publisher agent**: Evaluates if stories meet publication standards
- **Memory system**: Tracks iterations and feedback across multiple rounds

```bash
cd examples/multi_agent && ./run.sh
```

This example demonstrates:
- Complex multi-step workflows
- File I/O operations via MCP filesystem server
- Memory persistence for tracking state
- Iterative improvement based on feedback

### ⏰ Simple Time Operations (`examples/simple_time/`)
Basic time queries using MCP time server:

```bash
cd examples/simple_time && ./run_default_config.sh
# or
cd examples/simple_time && ./run_explicit_config.sh
```

Demonstrates:
- Automatic config discovery (`eunice.json`)
- Manual config specification
- Time/date MCP server integration

### 🖥️ Shell Command Execution (`examples/shell/`)
Execute shell commands through eunice using MCP shell server:

```bash
cd examples/shell && ./test.sh
```

Shows:
- Shell command execution via MCP
- Script automation
- System interaction capabilities

Each example includes its own configuration files and documentation, making them perfect starting points for your own eunice projects.

## Development

```bash
# Run directly with uv
uv run eunice.py "your prompt"

# Test specific features
uv run eunice.py --silent "quiet operation"
uv run eunice.py --list-models
uv run eunice.py --interact  # Start interactive mode

# Development workflow
make help           # Show all available commands
make dev            # Development mode
make reinstall      # Reinstall locally for testing
make clean          # Clean up temporary files

# Testing
make test-host      # Fast local tests (60% faster with optimizations)
make test-docker    # Docker environment tests
make build-and-test # Full build and test pipeline

# Publishing
make publish        # Push to Docker registry

# Uninstall
uv tool uninstall eunice
```

## Philosophy

eunice follows the principle of "sophisticated simplicity" - providing powerful agentic capabilities while maintaining a minimal, readable codebase that can be easily understood and modified.

## Getting Started

1. **Install eunice**: `uv tool install git+https://github.com/xeb/eunice`
2. **Set API keys**: Export your preferred AI service API key (OpenAI, Gemini, or Anthropic)
3. **Try basic usage**: `eunice "Hello, how are you?"`
4. **Explore examples**: Check out the `examples/` directory for practical use cases
5. **Create your config**: Copy `config.example.json` to `eunice.json` and customize

For detailed documentation, see [CLAUDE.md](CLAUDE.md).
