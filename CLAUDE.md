# Eunice - Development Guide

## About

Eunice is an agentic CLI runner written in Rust that provides a unified interface for multiple AI providers (OpenAI, Gemini, Anthropic Claude, and Ollama). It supports **multi-agent orchestration** where agents can invoke other agents as tools. It emphasizes "sophisticated simplicity".

## Architecture

### Provider System

The codebase uses a provider abstraction layer that routes requests to different AI APIs:

```
User Input → Provider Detection → Client → API Request → Response
```

**Special Case: Gemini API Dual Support**
- Most models: Use OpenAI-compatible API (`/v1beta/openai/`)
- `gemini-3-pro-preview`: Uses native Gemini API (`/v1beta/models/{model}:generateContent`)

### Key Components

1. **Provider Detection** (`src/provider.rs`)
   - Detects model → provider mapping
   - Handles model aliases (e.g., `sonnet` → `claude-sonnet-4-20250514`)
   - Sets `use_native_gemini_api` flag for special models

2. **Client** (`src/client.rs`)
   - HTTP client with provider-specific headers
   - Message format conversion (OpenAI ↔ Gemini)
   - 429 retry logic with 6-second backoff (DMN mode only)

3. **MCP Integration** (`src/mcp/`)
   - Manages multiple MCP server subprocesses (stdio) or HTTP connections
   - Supports two transports: stdio (command/args) and Streamable HTTP (url)
   - JSON-RPC communication
   - Tool discovery and routing
   - Failed server reporting to model

4. **DMN Mode** (`src/config.rs`)
   - Default Mode Network: Autonomous batch execution
   - Minimal tool set: shell + filesystem (interpret_image is built-in)
   - Shell provides access to grep, curl, wget, git, etc.
   - Includes comprehensive system instructions

5. **Agent Loop** (`src/agent.rs`)
   - Main conversation loop
   - Tool execution with spinners
   - Conversation history management
   - Built-in `interpret_image` tool for multimodal analysis

6. **Multi-Agent Orchestrator** (`src/orchestrator/`)
   - Manages agent configurations and prompts
   - Creates `invoke_*` tools for agent-to-agent calls
   - Filters MCP tools by agent permissions
   - Handles recursive agent invocation with depth tracking

## Testing

The project includes **30 unit tests** covering:
- Provider detection logic
- Message format conversions
- Response parsing
- Gemini API serialization/deserialization
- Multi-agent orchestrator logic

Run tests with:
```bash
cargo test
# or
make test
```

## Line Count and Binary Size Guidelines

When updating the codebase, **ALWAYS** update both metrics in README.md:

### Count Implementation Lines
```bash
for file in src/*.rs src/mcp/*.rs src/orchestrator/*.rs; do
  test_start=$(grep -n "^#\[cfg(test)\]" "$file" | cut -d: -f1 | head -1)
  if [ -n "$test_start" ]; then
    lines=$((test_start - 1))
  else
    lines=$(wc -l < "$file")
  fi
  total=$((total + lines))
done
echo "Total: $total lines"
```

### Check Binary Size
```bash
make binary-size
# or
cargo build --release && ls -lh target/release/eunice
```

**Important**: Update both values in README.md after any code changes.

## Development Workflow

### Adding a New Provider

1. Update `Provider` enum in `src/models.rs`
2. Add detection logic in `src/provider.rs::detect_provider()`
3. Handle authentication in `src/client.rs::new()`
4. Add provider-specific logic if needed
5. Add tests in `src/provider.rs`

### Adding a New Model

1. If using existing provider API format, just add to available models list
2. If using different API format (like gemini-3-pro-preview):
   - Add flag to `ProviderInfo`
   - Implement format conversion in `Client`
   - Add tests for conversions

### Publishing

Update version and publish:
```bash
make publish  # Bumps patch version and publishes to crates.io
```

## Native Gemini API Implementation

The `gemini-3-pro-preview` model uses a different API format:

**Request Format:**
```json
{
  "contents": [
    {
      "parts": [{"text": "user message"}],
      "role": "user"
    }
  ]
}
```

**Authentication:**
- Header: `x-goog-api-key: $GEMINI_API_KEY`
- URL: `https://generativelanguage.googleapis.com/v1beta/models/gemini-3-pro-preview:generateContent`

**Conversion Logic:**
- `Message::User` → role: "user"
- `Message::Assistant` → role: "model"
- `Message::Tool` → skipped (not supported)

## Multi-Agent Architecture

Eunice supports multi-agent orchestration where agents can invoke other agents as tools.

### Configuration

Agents are defined in `eunice.toml`:

```toml
[mcpServers.shell]
command = "mcpz"
args = ["server", "shell"]

[agents.root]
prompt = "You are the coordinator..."
tools = []                           # MCP servers this agent can use
can_invoke = ["worker"]              # Agents this agent can call

[agents.worker]
prompt = "agents/worker.md"          # Can be file path
tools = ["shell"]
can_invoke = []
```

### How It Works

1. When `[agents]` section exists, multi-agent mode is auto-enabled
2. Default agent is `root` (or specify with `--agent name`)
3. Each agent gets `invoke_*` tools for agents in `can_invoke`
4. Agent invocation is recursive with depth tracking
5. MCP tools are filtered per-agent based on `tools` list

### CLI Usage

```bash
eunice "task"                    # Uses root agent if agents configured
eunice --agent worker "task"     # Use specific agent
eunice --list-agents             # Show configured agents
```

## Image and PDF Interpretation

Eunice includes a built-in `interpret_image` tool for multimodal analysis of images and PDF documents.

### Enabling

- **DMN mode**: Auto-enabled with `--dmn`
- **Standalone**: Use `--images` flag

### Implementation (`src/agent.rs`)

1. `get_interpret_image_tool_spec()` - Returns tool definition
2. `execute_interpret_image()` - Reads file, base64 encodes, calls multimodal API
3. Tool is added when `enable_image_tool` flag is true

### Multimodal API Support (`src/client.rs`)

- `chat_completion_with_image()` method handles both:
  - Native Gemini API (via `inlineData`)
  - OpenAI-compatible API (via `image_url` content blocks)

### Supported Formats

- Images: PNG, JPEG, GIF, WebP
- Documents: PDF

### CLI Usage

```bash
eunice --images "Describe screenshot.png"
eunice --dmn "Analyze diagram.jpg and summarize it"
eunice --dmn "Extract text from document.pdf"
```

## Key Design Decisions

1. **OpenAI-Compatible as Default**: Most providers offer OpenAI-compatible APIs
2. **Per-Model API Selection**: Uses `use_native_gemini_api` flag for special cases
3. **Message Format Abstraction**: Internal `Message` enum converts to provider formats
4. **DMN Mode for Autonomy**: Automatic retry and continuous execution
5. **MCP for Extensibility**: Tools provided via Model Context Protocol servers
6. **Agents as Tools**: Agent-to-agent calls are just tool calls, reusing MCP infrastructure
7. **Built-in Tools**: Special tools like `interpret_image` are handled directly by the agent

## File Structure

```
src/
├── main.rs              - CLI entry, arg parsing, multi-agent detection
├── models.rs            - Data structures + AgentConfig
├── client.rs            - HTTP client, format conversions
├── mcp/
│   ├── mod.rs           - Module exports
│   ├── server.rs        - MCP subprocess (stdio transport)
│   ├── http_server.rs   - MCP HTTP client (Streamable HTTP transport)
│   └── manager.rs       - Tool routing with async state
├── orchestrator/
│   ├── mod.rs           - Module exports
│   └── orchestrator.rs  - Multi-agent coordination
├── provider.rs          - Provider detection
├── display.rs           - Terminal UI with indicatif spinners
├── interactive.rs       - Interactive REPL mode
├── agent.rs             - Single-agent loop with tool execution
├── config.rs            - Configuration loading
└── lib.rs               - Library exports

dmn_instructions.md      - DMN system instructions (embedded via include_str!)
```

## Dependencies

- **tokio**: Async runtime for HTTP and MCP communication
- **reqwest**: HTTP client with timeout support
- **serde/serde_json**: Serialization
- **clap**: CLI with aliases and env var support
- **colored**: Terminal colors
- **indicatif**: Progress spinners with braille characters
- **crossterm**: Terminal control
- **anyhow/thiserror**: Error handling
- **base64**: Image encoding for multimodal requests

## Contributing

When adding features:
1. Write implementation code first
2. Add unit tests
3. Update line counts and binary size:
   - Count implementation lines (see "Line Count Guidelines")
   - Run `make binary-size` to get release binary size
   - Update both values in README.md
4. Update this CLAUDE.md file if structure changed
5. Run `make test` to verify
6. Run `cargo build --release` to check for warnings

## Version History

- **0.2.10**: Schema sanitization - removes `x-*` extension fields for Gemini compatibility
- **0.2.9**: Verbose tool schema output for debugging provider compatibility
- **0.2.8**: Configurable MCP timeout (`timeout` in config), default 10 minutes
- **0.2.7**: Verbose MCP debugging (`--verbose`), tool name sanitization for Gemini compatibility
- **0.2.6**: Documentation updates for PDF support
- **0.2.5**: PDF understanding support via `interpret_image` tool
- **0.2.4**: Minimal DMN (shell + filesystem only), Design Goals section, curl/wget for web
- **0.2.3**: Image interpretation via `--images` flag and `interpret_image` built-in tool
- **0.2.2**: Streamable HTTP MCP transport, failed server reporting to model
- **0.2.1**: Embedded llms.txt/llms-full.txt via --llms-txt/--llms-full-txt flags
- **0.2.0**: Multi-agent orchestration, agents can invoke other agents as tools
- **0.1.12**: TOML config support, mcpz preference for DMN mode
- **0.1.11**: Default model changed to gemini-3-pro-preview, auto-prompt discovery
- **0.1.10**: Thinking indicator with elapsed time
- **0.1.9**: Fixed MCP server timeout (stderr deadlock)
- **0.1.1**: Added native Gemini API support, 429 retry, spinners, unit tests
- **0.1.0**: Initial release with multi-provider support and MCP integration

## Publishing

When the user says "publish":
1. Run `cargo test` - all tests must pass
2. Update LOC and binary size in README.md
3. Git commit with descriptive message
4. Git tag (e.g., `v0.2.1`)
5. Git push (with tags)
6. `cargo publish`