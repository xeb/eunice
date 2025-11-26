# Component: MCP (Model Context Protocol)
**Path:** `src/mcp/`
**Last Analyzed:** 2025-11-26 12:25
**Primary Author(s):** Mark Kockerbeck

## Purpose
Implements the Model Context Protocol (MCP) client to extend LLM capabilities with external tools. The MCP module manages multiple tool servers as child processes, communicates via JSON-RPC over stdin/stdout, and provides a unified interface for tool discovery and execution.

## Structure
| File | Lines | Purpose |
|------|-------|---------|
| `mod.rs` | 4 | Module exports, re-exports `McpManager` |
| `manager.rs` | 336 | Server lifecycle management, tool routing, lazy loading |
| `server.rs` | 288 | Individual server communication, JSON-RPC protocol |

**Total:** 628 lines

## Key Patterns

### 1. Lazy Server Initialization
Servers are spawned immediately but initialized asynchronously in background tasks. This allows fast startup while servers warm up in parallel.

```rust
pub enum ServerState {
    Initializing(JoinHandle<Result<McpServer>>),
    Ready(McpServer),
    Failed(String),
}
```

### 2. Two-Phase Spawn Pattern
`SpawnedServer::spawn()` is synchronous and fast (just spawns the process), while `initialize()` is async and handles the JSON-RPC handshake.

### 3. Prefix-Based Tool Routing
Tools are namespaced by server name (e.g., `shell_run_command`). The manager uses longest-prefix matching to route calls:
```rust
tool_name.strip_prefix(&format!("{}_", server_name))
```

### 4. Retry with Exponential Backoff
Server initialization retries up to 5 times with exponential backoff (100ms → 200ms → 400ms → 800ms → 1000ms) to handle slow-starting servers.

### 5. JSON-RPC over stdio
Communication uses newline-delimited JSON-RPC 2.0 over stdin/stdout. Non-JSON lines are silently skipped (allows servers to emit debug output).

## Dependencies

**Internal:**
- `crate::models` - JSON-RPC types, MCP data structures, Tool definitions

**External (Rust crates):**
- `tokio` - Async runtime, process spawning, channels
- `serde_json` - JSON serialization/deserialization
- `anyhow` - Error handling with context

## API Surface

### McpManager
- `new()` - Create empty manager
- `start_servers_background(&mut self, config, silent)` - Non-blocking server startup
- `await_all_servers(&mut self)` - Wait for pending initializations
- `get_tools() -> Vec<Tool>` - Get all tools from ready servers
- `execute_tool(tool_name, args) -> Result<String>` - Route and execute a tool call
- `get_server_info() -> Vec<(name, count, tool_names)>` - Status display
- `shutdown(&mut self)` - Graceful termination

### McpServer
- `call_tool(name, args) -> Result<String>` - Execute single tool
- `stop(&mut self)` - Terminate server process

## Protocol Implementation

The MCP handshake follows this sequence:
1. Send `initialize` request with client info
2. Receive capabilities response
3. Send `notifications/initialized`
4. Send `tools/list` request
5. Receive tools with schemas
6. (Ready for tool calls)

Tool calls use `tools/call` method with name and arguments.

## Design Decisions

### stderr → null
Server stderr is piped to null to prevent deadlock. If MCP servers write too much to stderr without a reader, the buffer fills and the server blocks.
```rust
.stderr(std::process::Stdio::null())
```
**Context:** This was fixed in v0.1.9 after timeout issues.

### 60-Second Tool Timeout
Read operations have a 60-second timeout, which is generous for any reasonable MCP tool operation.

### Graceful Shutdown
Kill signal with 2-second wait. If timeout, continues without error (process may have already exited).

## Concerns
- **No stderr visibility:** Debugging MCP servers is harder since stderr is discarded
- **No tool validation:** Tool schemas are passed through but not validated before calls
- **Single-threaded tool calls:** Each McpServer requires `&mut self` for calls (no concurrent tool invocations per server)

## History
| Commit | Description |
|--------|-------------|
| 5bed40a | v0.1.9: Fix stderr deadlock causing timeouts |
| 10aff86 | v0.1.8: Gemini 3 Pro refactors |
| 1a55aab | v0.1.5: Add lazy loading for MCP servers |
| a3c2b17 | Initial Rust migration |

## Notes
- MCP is an Anthropic-originated protocol for tool use standardization
- Protocol version: `2024-11-05`
- Default servers (DMN mode): shell, filesystem, text-editor, grep, memory, brave-search
- Server configs can be loaded from JSON files or use built-in defaults
