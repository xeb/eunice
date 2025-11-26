# Project Overview: Eunice

**Type:** CLI Application
**Language:** Rust
**Path:** `/media/xeb/GreyArea/projects/eunice`
**Description:** An agentic CLI runner in Rust with unified support for OpenAI, Gemini, Claude, and Ollama via OpenAI-compatible APIs. Features MCP integration and an autonomous "DMN" (Default Mode Network) mode.

## Key Directories
- `src/`: Core application logic
- `examples/`: Usage examples
- `target/`: Build artifacts

## Build & Run
- **Build:** `cargo build --release`
- **Run:** `cargo run -- [args]`
- **Test:** `cargo test` (assumed standard Rust)

## Recent Activity
- **v0.1.9:** Fix MCP server timeout (stderr deadlock)
- **v0.1.8:** Gemini 3 Pro refactors
- **v0.1.7:** Gemini native API schema sanitization

## Primary Technologies
- Rust (Edition 2021)
- MCP (Model Context Protocol)
- LLM APIs (OpenAI, Gemini, Anthropic, Ollama)
