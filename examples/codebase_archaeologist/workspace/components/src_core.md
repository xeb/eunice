# Component: Core (src)
**Path:** `src/`
**Last Analyzed:** 2025-11-25
**Primary Author(s):** Mark Kockerbeck (implied from Cargo.toml)

## Purpose
The core application logic for Eunice, implementing the agentic loop, LLM provider abstraction, and Model Context Protocol (MCP) integration.

## Structure
- `main.rs`: CLI entry point, argument parsing, and initialization.
- `lib.rs`: Library interface exporting modules.
- `agent.rs`: Main agent loop (tools execution, conversation history).
- `provider.rs`: LLM provider detection and configuration (OpenAI, Gemini, Anthropic, Ollama).
- `mcp/`: Model Context Protocol implementation.
  - `manager.rs`: Lazy loading and management of multiple MCP servers.
  - `server.rs`: JSON-RPC communication with MCP servers over stdio.
- `client.rs`: HTTP client for LLM APIs.
- `config.rs`: Configuration loading (including DMN presets).
- `display.rs`: Terminal UI utilities (spinners, colors).
- `interactive.rs`: Interactive session handling.
- `models.rs`: Shared data structures.

## Key Patterns
- **Unified Provider API**: Abstracts differences between OpenAI, Gemini, etc., usually normalizing to OpenAI-like schemas.
- **Lazy MCP Loading**: Servers are started in the background (`Initializing` state) and awaited only when needed.
- **DMN Mode**: "Default Mode Network" - a specific flag for autonomous operation with pre-configured tools.
- **Agent Loop**:
  1. Append user prompt.
  2. Fetch available tools.
  3. Call LLM.
  4. Append response.
  5. If tool calls, execute and recurse (implied).

## Dependencies
**Internal:**
- `crate::client`
- `crate::mcp`
- `crate::models`

**External:**
- `tokio` (Async runtime)
- `reqwest` (HTTP)
- `serde` (JSON)
- `clap` (CLI args)
- `crossterm`, `indicatif` (UI)

## Concerns
- **Code Duplication**: `main.rs` and `lib.rs` both declare modules, potentially causing double compilation.
- **Hardcoded Models**: `provider.rs` contains hardcoded mappings for future model versions (e.g., "claude-sonnet-4-20250514").
- **No TODOs**: Explicit "TODO" comments are missing, which might mask debt.

## Notes
- The project migrated to Rust from a previous version (likely Python or Node) in commit `a3c2b17`.
- Emphasis on "sophisticated simplicity" and small binary size.
