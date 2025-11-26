# Project: Eunice
**Type:** CLI Tool / Agent Runner
**Language:** Rust
**Frameworks:** Tokio, Clap, Reqwest, Serde
**Version:** 0.1.9 (as of 2025-11-26)

## Overview
Eunice is a "sophisticated simplicity" CLI runner for interacting with LLMs (OpenAI, Gemini, Claude, Ollama) via a unified API. It supports the Model Context Protocol (MCP) for tool extension and features a "Default Mode Network" (DMN) for autonomous batch execution.

## Key Markers
- **Cargo.toml**: Defines the Rust project and dependencies.
- **Makefile**: Build and publish automation.
- **dmn_instructions.md**: System instructions for the autonomous mode.
- **src/**: Core source code (~2.4k lines).

## Key Directories
- `src/`: Application logic.
    - `mcp/`: MCP server handling.
- `examples/`: Example agents and configurations.

## Build & Test
- Build: `cargo build --release`
- Install: `cargo install --path .`
