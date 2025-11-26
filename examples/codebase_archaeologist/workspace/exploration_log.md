## [2025-11-26 11:58] Explored: Core (src)
- Files analyzed: 12 Rust files
- Patterns found: Unified Client, Lazy Loading, Async/Await
- Concerns: client.rs size (706 lines)
- Key insight: A single monolithic Client struct handles all LLM providers, with special casing for Gemini.

## [2025-11-26 12:25] Explored: MCP (src/mcp/)
- Files analyzed: 3 Rust files (628 lines total)
- Patterns found: Lazy initialization, Two-phase spawn, Prefix routing, Exponential backoff, JSON-RPC over stdio
- Concerns: 3 (stderr invisibility, no tool validation, single-threaded per server)
- Key insight: Clean separation between spawning (sync) and initialization (async) enables parallel server startup without blocking.
