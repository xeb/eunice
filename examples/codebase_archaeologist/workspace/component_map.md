# Component Map

```
eunice/
├── CLI Entry (main.rs)
│   ├── Args parsing (clap)
│   ├── Mode selection: --dmn, --interactive, one-shot
│   └── Config loading (eunice.json or embedded)
│
├── LLM Client Layer
│   ├── client.rs ─────────────────────────────────────────┐
│   │   └── Unified API for OpenAI/Gemini/Claude/Ollama    │
│   ├── provider.rs                                         │
│   │   └── Provider detection and URL/env configuration   │
│   └── models.rs                                           │
│       └── Message, Tool, API request/response types      │
│                                                           │
├── Agent System ◄──────────────────────────────────────────┤
│   ├── agent.rs                                            │
│   │   └── Core loop: LLM → tools → results → repeat      │
│   ├── interactive.rs                                      │
│   │   └── REPL wrapper with DMN injection                │
│   └── config.rs                                           │
│       ├── DMN MCP server configuration (hardcoded)       │
│       └── DMN_INSTRUCTIONS (compiled-in prompt)          │
│                                                           │
├── MCP Layer (Tool Execution) ◄────────────────────────────┤
│   ├── mcp/mod.rs                                          │
│   │   └── McpManager: server lifecycle, tool routing     │
│   ├── mcp/client.rs                                       │
│   │   └── JSON-RPC over stdio                            │
│   └── mcp/server.rs                                       │
│       └── Server spawn, init, reconnect                  │
│                                                           │
└── Display Layer                                           │
    └── display.rs                                          │
        └── Spinners, tool output, progress indicators     │
```

## Data Flow (DMN Mode)

```
User Prompt
    │
    ▼
┌─────────────────┐
│ DMN_INSTRUCTIONS│ ← Embedded system prompt
│ + prompt wrap   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    agent.rs     │ ← Agent loop
│   run_agent()   │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌───────┐  ┌─────────┐
│client │  │   MCP   │
│.rs    │  │ manager │
└───┬───┘  └────┬────┘
    │           │
    ▼           ▼
 OpenAI    MCP Servers
 Gemini    (shell, fs,
 Claude     grep, etc.)
 Ollama
```

## Component Dependencies

| Component | Depends On | Depended By |
|-----------|------------|-------------|
| main.rs | all | - |
| client.rs | provider.rs, models.rs | agent.rs, interactive.rs |
| agent.rs | client.rs, mcp/, display.rs | main.rs, interactive.rs |
| mcp/ | models.rs | agent.rs |
| config.rs | models.rs | main.rs, interactive.rs |
| display.rs | - | agent.rs, main.rs |

## Module Boundaries

### Public APIs
- `client::Client` - LLM communication
- `agent::run_agent()` - Single agent execution
- `interactive::interactive_mode()` - REPL mode
- `mcp::McpManager` - Tool orchestration
- `config::get_dmn_mcp_config()` - Default tool config
- `config::DMN_INSTRUCTIONS` - System prompt

### Internal Only
- Provider detection logic
- Gemini response conversion
- MCP JSON-RPC protocol details
