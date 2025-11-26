# Eunice - Development Guide

## About

Eunice is an agentic CLI runner written in Rust that provides a unified interface for multiple AI providers (OpenAI, Gemini, Anthropic Claude, and Ollama). It emphasizes "sophisticated simplicity" with **2,433 lines of implementation code** (excluding tests) and a **3.6MB release binary**.

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
   - Manages multiple MCP server subprocesses
   - JSON-RPC communication
   - Tool discovery and routing

4. **DMN Mode** (`src/config.rs`)
   - Default Mode Network: Autonomous batch execution
   - Pre-configured with 7 MCP servers
   - Includes comprehensive system instructions

5. **Agent Loop** (`src/agent.rs`)
   - Main conversation loop
   - Tool execution with spinners
   - Conversation history management

## Testing

The project includes **23 unit tests** covering:
- Provider detection logic
- Message format conversions
- Response parsing
- Gemini API serialization/deserialization

Run tests with:
```bash
cargo test
# or
make test
```

## Line Count and Binary Size Guidelines

When updating the codebase, **ALWAYS** update both metrics in README.md:

### Current Metrics
- **Implementation lines**: 2,475 lines (excluding tests)
- **Binary size**: 3.6MB (release build)

### Count Implementation Lines
```bash
for file in src/*.rs src/mcp/*.rs; do
  test_start=$(grep -n "^#\[cfg(test)\]" "$file" | cut -d: -f1)
  if [ -n "$test_start" ]; then
    echo "$file: $((test_start - 1)) lines"
  else
    echo "$file: $(wc -l < "$file") lines"
  fi
done
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

## Key Design Decisions

1. **OpenAI-Compatible as Default**: Most providers offer OpenAI-compatible APIs
2. **Per-Model API Selection**: Uses `use_native_gemini_api` flag for special cases
3. **Message Format Abstraction**: Internal `Message` enum converts to provider formats
4. **DMN Mode for Autonomy**: Automatic retry and continuous execution
5. **MCP for Extensibility**: Tools provided via Model Context Protocol servers

## File Structure

```
src/
├── main.rs (239)          - CLI entry, arg parsing
├── models.rs (362)        - Data structures + Gemini response types
├── client.rs (518)        - HTTP client, format conversions
├── mcp/
│   ├── server.rs (288)    - MCP subprocess with lazy loading
│   └── manager.rs (275)   - Tool routing with async state
├── provider.rs (245)      - Provider detection
├── display.rs (210)       - Terminal UI with indicatif spinners
├── interactive.rs (112)   - Interactive REPL mode
├── agent.rs (133)         - Agent loop with tool execution
├── config.rs (89)         - Configuration loading
└── lib.rs (8)             - Library exports

dmn_instructions.md (188)  - DMN system instructions (embedded via include_str!)

Total: 2,475 lines (implementation) + 188 lines (embedded instructions)
Binary: 3.6MB (release build)
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

- **0.1.1**: Added native Gemini API support, 429 retry, spinners, unit tests
- **0.1.0**: Initial release with multi-provider support and MCP integration
- When I say "publish" all by itself, update the LOC and Binary size in the README, then write a git commit message, publish the crate and push the git updates. All at once.
- Oh when I say "publish" do what I said before but also add a git tag and be sure to run tests and make sure the release builds