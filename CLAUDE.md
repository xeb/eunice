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
   - Minimal tool set: shell + filesystem + browser (interpret_image is built-in)
   - Browser automation (optional, requires Chrome and mcpz)
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
- `Message::Tool` → converted to `functionResponse` (user role)

## Multi-Agent Architecture

Eunice supports multi-agent orchestration where agents can invoke other agents as tools.

### Configuration

Agents are defined in `eunice.toml`:

```toml
[mcpServers.shell]
command = "mcpz"
args = ["server", "shell"]

# Global tool filtering (optional)
allowedTools = ["shell_*"]           # Whitelist: only these patterns
deniedTools = ["*_background"]       # Blacklist: exclude these patterns

[agents.root]
prompt = "You are the coordinator..."
tools = []                           # Tool patterns this agent can use
can_invoke = ["worker"]              # Agents this agent can call

[agents.worker]
prompt = "agents/worker.md"          # Can be file path
model = "gemini-3-flash-preview"     # Optional: use different model (faster/cheaper)
tools = ["shell_*"]                  # Supports wildcards: shell_*, *_read, etc.
can_invoke = []
```

### Per-Agent Models

Each agent can specify its own model via the optional `model` field:

- **Default**: Agents without `model` use the `--model` flag value (or auto-detected default)
- **Override**: Specify `model = "gemini-3-flash-preview"` for faster/cheaper execution
- **Validation**: All agent models are validated at startup (API key + model availability)

Use faster models (e.g., `gemini-3-flash-preview`) for simpler agents and reserve powerful models for coordinators.

### How It Works

1. When `[agents]` section exists, multi-agent mode is auto-enabled
2. Default agent is `root` (or specify with `--agent name`)
3. Each agent gets `invoke_*` tools for agents in `can_invoke`
4. Agent invocation is recursive with depth tracking
5. MCP tools are filtered per-agent based on `tools` list
6. Each agent uses its own model (or default if not specified)

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

## Web Search

Eunice includes a built-in `search_query` tool for web searches using Gemini models with Google Search grounding.

### Enabling

- **DMN mode**: Auto-enabled with `--dmn`
- **Standalone**: Use `--search` flag

### Implementation (`src/agent.rs`)

1. `get_search_query_tool_spec()` - Returns tool definition with model enum
2. `execute_search_query()` - Makes Gemini API request with `google_search` tool
3. Tool is added when `enable_search_tool` flag is true

### Model Selection

- `flash` (gemini-2.5-flash): Quick knowledge queries, fast and cheap
- `pro` (gemini-2.5-pro): Medium complexity queries requiring deeper analysis
- `pro_preview` (gemini-3-pro-preview): Hardest queries requiring maximum reasoning

### CLI Usage

```bash
eunice --search --no-mcp "What are the latest AI developments?"
eunice --dmn "Search for the current weather in Tokyo"
```

## Research Mode

Eunice includes a built-in `--research` mode for multi-agent research orchestration using Gemini with Google Search grounding.

### Enabling

Use `--research` flag (requires `GEMINI_API_KEY`):

```bash
eunice --research "Best laptops of 2025"
eunice --research --tui  # TUI mode for interactive research
```

### Architecture

Research mode uses 4 embedded agents following the orchestrator-workers pattern:

1. **root** (coordinator): Breaks research into subtopics, delegates to workers, manages workflow
2. **researcher**: Uses `search_query` tool with `pro_preview` model, saves notes to `research_notes/`
3. **report_writer**: Reads research notes, synthesizes into reports in `reports/`
4. **evaluator**: Reviews reports, returns APPROVED or NEEDS_REVISION (one revision cycle)

### Implementation (`src/config.rs`)

- `get_research_mcp_config()` - Returns embedded agent configuration (filesystem + browser MCP servers)
- `has_gemini_api_key()` - Checks for required API key
- Embedded prompts: `RESEARCH_LEAD_PROMPT`, `RESEARCH_RESEARCHER_PROMPT`, etc.
- Researcher agent has access to browser tools (optional, for JavaScript-heavy pages)

### Workflow

1. User provides research topic
2. Root agent breaks into 2-4 subtopics
3. Researcher agents search web and save notes
4. Report writer synthesizes findings
5. Evaluator reviews (one revision if needed)
6. Final report in `reports/` directory

### CLI Flags

- `--research`: Enable research mode (conflicts with `--dmn`)
- `--research --interact`: Interactive research sessions
- `--research --list-agents`: Show embedded agents
- `--research --config eunice.toml`: Merge MCP servers from config (agents ignored)

## Key Design Decisions

1. **OpenAI-Compatible as Default**: Most providers offer OpenAI-compatible APIs
2. **Per-Model API Selection**: Uses `use_native_gemini_api` flag for special cases
3. **Message Format Abstraction**: Internal `Message` enum converts to provider formats
4. **DMN Mode for Autonomy**: Automatic retry and continuous execution
5. **MCP for Extensibility**: Tools provided via Model Context Protocol servers
6. **Agents as Tools**: Agent-to-agent calls are just tool calls, reusing MCP infrastructure
7. **Built-in Tools**: Special tools like `interpret_image` and `search_query` are handled directly by the agent

## File Structure

```
src/
├── main.rs              - CLI entry, arg parsing, multi-agent detection
├── models.rs            - Data structures + AgentConfig + WebappConfig
├── client.rs            - HTTP client, format conversions
├── mcp/
│   ├── mod.rs           - Module exports
│   ├── server.rs        - MCP subprocess (stdio transport)
│   ├── http_server.rs   - MCP HTTP client (Streamable HTTP transport)
│   └── manager.rs       - Tool routing with async state
├── orchestrator/
│   ├── mod.rs           - Module exports
│   └── orchestrator.rs  - Multi-agent coordination
├── webapp/
│   ├── mod.rs           - Module exports
│   ├── server.rs        - Axum web server setup
│   └── handlers.rs      - HTTP/SSE request handlers
├── provider.rs          - Provider detection
├── display.rs           - Terminal UI output
├── interactive.rs       - Interactive REPL mode
├── agent.rs             - Single-agent loop with tool execution
├── config.rs            - Configuration loading
└── lib.rs               - Library exports

webapp/
└── index.html           - Embedded HTML/CSS/JS frontend (synth minimal aesthetic)

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

- **0.2.68**: Multi-agent: required `description` field for agents, visible invoke calls in CLI/TUI/webapp
- **0.2.67**: Fix session switching: respect session_id from request instead of always using most recent
- **0.2.66**: Per-agent models: agents can specify their own `model` in config, validated at startup
- **0.2.59**: Webapp SQLite session persistence (if mcpz installed), hamburger menu with session list, cyberpunk session names
- **0.2.58**: Display tool call arguments in grey underneath tool name in TUI and CLI output
- **0.2.57**: Webapp event replay: reconnect to see events that happened while browser was closed; agents continue running when tab closes
- **0.2.56**: HTTP MCP detailed errors (timeout/status/body), remove --interact flag (TUI auto-launches when no prompt given), fix TUI exit prompt
- **0.2.55**: TUI mode: Fix line rendering (use crossterm directly instead of SharedWriter for in-place editing)
- **0.2.54**: DMN mode is now a proper agent (shows in `--dmn --list-agents`) consistent with `--research`
- **0.2.53**: TUI mode: Bracketed paste support for multiline paste (via Ctrl+Shift+V or terminal paste)
- **0.2.52**: TUI mode: Escape/Ctrl+C cancellation support to stop generation mid-response
- **0.2.51**: Add gemini-3-flash-preview as new default model; add gemini-3-flash and gemini-3-pro aliases
- **0.2.50**: TUI DisplaySink refactor - all output routed through SharedWriter for proper terminal coordination
- **0.2.49**: TUI mode (`--tui`) using r3bl_tui for enhanced terminal interface with command menu, trimmed response output
- **0.2.48**: Simplified display output
- **0.2.47**: Browser `is_available` tool for pre-checking Chrome availability
- **0.2.46**: Browser automation MCP server for DMN and Research modes (optional, requires Chrome and mcpz)
- **0.2.42**: Webapp: tabbed Preview/Code view for HTML and Markdown responses
- **0.2.41**: Webapp: add console logging for debugging (sessions, queries, LLM calls, tool execution); improve spinner terminal cleanup
- **0.2.40**: Webapp: display version number in status bar footer
- **0.2.39**: Webapp: fix tool count in status bar to include built-in tools; add session history restoration on page refresh
- **0.2.38**: Fix spinner terminal artifacts: explicit line clear after stopping spinner to prevent whitespace issues
- **0.2.37**: Webapp: improved error display (HTTP errors, parse errors, connection closed without response)
- **0.2.36**: Research mode: `--config` can now be used with `--research` to merge MCP servers (agents ignored)
- **0.2.35**: Documentation: --webapp examples with custom host/port configuration in README and llms-full.txt
- **0.2.34**: Webapp: server-side session management with NEW button, localStorage persistence, mobile zoom fix
- **0.2.33**: Webapp: autoscroll fix, mobile responsive design, markdown/HTML rendering; Interactive: multiline input fix
- **0.2.32**: Webapp multi-turn sessions with conversation history and automatic context compaction
- **0.2.31**: Webapp config endpoint includes built-in tools (interpret_image, search_query) when enabled
- **0.2.30**: Webapp enhancements: Config modal, agent display in status bar, auto-scroll fix
- **0.2.29**: Webapp mode via `--webapp` flag with browser-based interface and real-time SSE streaming
- **0.2.27**: Research mode via `--research` flag with built-in multi-agent orchestration (requires GEMINI_API_KEY)
- **0.2.26**: Web search tool via `--search` flag using Gemini with Google Search grounding
- **0.2.24**: Enhanced verbose logging for HTTP MCP connections (shows request/response bodies, status, content-type)
- **0.2.23**: Auto context compression when RESOURCE_EXHAUSTED error occurs (DMN mode)
- **0.2.22**: Added SSE response parsing for HTTP MCP servers (FastMCP compatibility)
- **0.2.21**: Fixed HTTP MCP Accept header to include both `application/json` and `text/event-stream` per MCP spec
- **0.2.12**: Fixed Gemini native API tool response handling (wraps non-object responses)
- **0.2.11**: Short prefix system for tool names (m0_, m1_) to stay under Gemini's 64-char limit
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

## Releasing

When the user says "publish" or "release":
1. Run `cargo test` - all tests must pass
2. Update LOC and binary size in README.md
3. Bump version in Cargo.toml
4. Git commit with descriptive message
5. Git push
6. **IMPORTANT**: After git push completes, run `eunice --update` to install the new version from GitHub