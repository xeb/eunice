# Component: Core (src)
**Path:** src/
**Last Analyzed:** 2025-11-26
**Primary Language:** Rust

## Purpose
Contains the core logic for the Eunice CLI, including the main execution loop, provider abstractions, and MCP integration.

## Structure
- **client.rs** (706 lines): The heavyweight handling HTTP communications. Implements a unified client for OpenAI, Claude, Gemini, and Ollama. Note the `use_native_gemini_api` flag.
- **models.rs** (460 lines): Defines the unified data model (Messages, ToolCalls) and provider-specific schemas (GeminiRequest).
- **mcp/**:
  - **manager.rs** (336 lines): Manages tool routing and server state.
  - **server.rs** (288 lines): Handles subprocess lifecycle (spawn, lazy load).
- **provider.rs** (332 lines): Logic for detecting available providers and API keys.
- **main.rs** (239 lines): CLI entry point using `clap`.
- **display.rs** (210 lines): UI handling with `indicatif` spinners.
- **agent.rs** (133 lines): The high-level agent loop logic.
- **interactive.rs** (112 lines): specific logic for interactive mode.
- **config.rs** (81 lines): Loads `eunice.json` configuration.

## Key Patterns
- **Unified Client**: A single `Client` struct abstracts differences between providers, though special handling for Gemini exists.
- **Lazy MCP Loading**: Implemented in `mcp/server.rs` to improve startup time.
- **Async/Await**: Heavy usage of Tokio for async I/O.
- **Error Handling**: Uses `anyhow::Result` and `thiserror` throughout.

## Dependencies
- **Internal**: Strong coupling between `client.rs` and `models.rs`. `agent.rs` orchestrates everything.
- **External**: `reqwest` (HTTP), `tokio` (runtime), `clap` (CLI), `serde` (JSON).

## Concerns
- **Complexity in Client**: `client.rs` is large and handles multiple provider quirks. It might be a candidate for refactoring into a trait-based provider system if it grows further.
- **Cleanliness**: Remarkable absence of TODO/FIXME comments in the codebase.

## Notes
- The "sophisticated simplicity" philosophy is evident in keeping the file count low, even if some files are getting large.
