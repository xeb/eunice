# Component: DMN & Agent System
**Path:** `src/config.rs`, `src/agent.rs`, `src/interactive.rs`, `dmn_instructions.md`
**Last Analyzed:** 2025-11-26 13:46
**Primary Author(s):** xeb (based on git history)

## Purpose
The DMN (Default Mode Network) and Agent system provides Eunice's autonomous execution capability. It transforms a simple LLM client into a self-directing agent that can complete complex, multi-step tasks without user intervention.

## Structure

| File | Lines | Description |
|------|-------|-------------|
| `src/config.rs` | 81 | DMN MCP config (hardcoded servers), config file loading, embeds DMN instructions |
| `src/agent.rs` | 133 | Core agent loop: LLM → tool calls → results → repeat |
| `src/interactive.rs` | 107 | Interactive REPL wrapper around agent with DMN injection |
| `dmn_instructions.md` | 279 | System prompt defining autonomous behavior and MCP tool usage |

## Key Patterns

### 1. Embedded System Instructions
```rust
pub const DMN_INSTRUCTIONS: &str = include_str!("../dmn_instructions.md");
```
The DMN system prompt is compiled into the binary via `include_str!`. This ensures the instructions are always available and versioned with the code.

### 2. Prompt Injection Pattern
DMN mode wraps user prompts with instructions:
```rust
format!(
    "{}\\n\\n---\\n\\n# USER REQUEST\\n\\n{}\\n\\n---\\n\\nYou are now in DMN... autonomous batch mode...",
    DMN_INSTRUCTIONS, prompt
)
```
The wrapper includes clear delimiters (`---`) to separate system instructions from user content.

### 3. One-Time Injection Guard
In interactive mode, DMN instructions are injected only on the first prompt:
```rust
let mut dmn_injected = false;
// Later...
if dmn && !*dmn_injected {
    *dmn_injected = true;
    // inject instructions
}
```
This prevents bloating conversation history with repeated instructions.

### 4. Agent Loop (Agentic Execution)
```rust
loop {
    let response = client.chat_completion(...).await?;
    // Add assistant message to history
    
    let Some(tool_calls) = &choice.message.tool_calls else {
        break;  // No tools = done
    };
    
    for tool_call in tool_calls {
        let result = manager.execute_tool(tool_name, args).await;
        conversation_history.push(Message::Tool { ... });
    }
    // Loop back to get LLM's next action
}
```
The loop continues until the LLM responds without tool calls, indicating task completion.

### 5. Hardcoded MCP Server Configuration
DMN mode uses a preset collection of MCP servers:
- **shell**: Command execution via `uvx mcp-server-shell`
- **filesystem**: File operations via `@modelcontextprotocol/server-filesystem`
- **text-editor**: Line-based editing via `mcp-text-editor`
- **grep**: Code search via `mcp-ripgrep`
- **memory**: Persistent knowledge graph via `@modelcontextprotocol/server-memory`
- **web**: Search via `@brave/brave-search-mcp-server`

### 6. Rate Limit Resilience (DMN-specific)
```rust
if status.as_u16() == 429 && dmn_mode && attempt == 1 {
    eprintln!("⏳ Rate limit hit (429). DMN mode: retrying in 6 seconds...");
    tokio::time::sleep(Duration::from_secs(6)).await;
    continue;
}
```
In DMN mode (but not regular mode), 429 errors trigger automatic retry to avoid losing agent state.

## Dependencies
**Internal:**
- `client.rs` - LLM API communication
- `mcp/` - Tool execution via MCP servers
- `models.rs` - Message, Tool types
- `display.rs` - UI output (spinners, tool display)

**External:**
- `tokio` - async runtime
- `anyhow` - error handling
- `serde_json` - tool argument parsing

## Architecture Flow

```
                        ┌─────────────────────┐
                        │    main.rs / CLI    │
                        └──────────┬──────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │ --dmn flag         │ --interactive      │
              ▼                    ▼                    │
    ┌─────────────────┐   ┌───────────────────┐        │
    │ get_dmn_mcp_    │   │ interactive_mode()│        │
    │ config()        │   └─────────┬─────────┘        │
    └────────┬────────┘             │                  │
             │                      ▼                  │
             │            ┌─────────────────┐          │
             │            │ inject_dmn_     │          │
             │            │ instructions()  │          │
             │            └────────┬────────┘          │
             │                     │                   │
             └──────────┬──────────┘                   │
                        ▼                              │
              ┌─────────────────┐                      │
              │  run_agent()    │ ◄────────────────────┘
              └────────┬────────┘
                       │
         ┌─────────────┴─────────────┐
         │         Agent Loop        │
         │  ┌─────────────────────┐  │
         │  │ chat_completion()   │  │
         │  └──────────┬──────────┘  │
         │             ▼             │
         │  ┌─────────────────────┐  │
         │  │ tool_calls?         │──┼──► No → Exit loop
         │  └──────────┬──────────┘  │
         │             │ Yes         │
         │             ▼             │
         │  ┌─────────────────────┐  │
         │  │ execute_tool()      │  │
         │  │ (via McpManager)    │  │
         │  └──────────┬──────────┘  │
         │             │             │
         │         Loop back         │
         └───────────────────────────┘
```

## Concerns

1. **Hardcoded MCP Servers** (config.rs:7-62)
   - Server packages are hardcoded with no version pinning
   - `npx -y` and `uvx` fetch latest versions each time
   - Could break if upstream packages change

2. **Single Retry for Rate Limits**
   - Only retries once with fixed 6-second delay
   - May not be sufficient for sustained rate limiting
   - No exponential backoff

3. **No Conversation Length Limit**
   - Agent loop adds to history indefinitely
   - Long-running tasks could hit context limits

## Notes

### Design Philosophy
The DMN system embodies "sophisticated simplicity" - complex autonomous behavior emerges from a simple loop pattern. The agent doesn't need specialized planning modules; it relies on the LLM's reasoning capability guided by detailed system instructions.

### Historical Context
- Migrated from Python (commit `a3c2b17`)
- Major refactor in `c0ce5d9` reduced codebase to ~1,950 lines while adding Gemini native API
- DMN instructions are heavily inspired by Claude's "agentic coding" prompts

### DMN Instructions Content (dmn_instructions.md)
The 279-line system prompt defines:
- Core mandates (autonomous execution, convention adherence, no assumptions)
- Primary workflows (software engineering, new applications)
- Operational guidelines (tone, security, tool usage)
- MCP tool reference (how to use each tool type)
- Expected behaviors and final reminders

Key phrases that shape behavior:
- "Execute ALL steps without stopping for confirmation"
- "Make reasonable decisions independently"
- "NEVER assume a library/framework is available"
- "You are an agent - keep going until the user's query is completely resolved"
