# Next Steps

## Priority Explorations

1. **Deep Dive: Client Architecture** (High Priority)
   - `src/client.rs` (706 lines) is the largest file
   - Understand how it normalizes OpenAI vs Gemini Native APIs
   - Map the streaming/non-streaming code paths
   
2. **Examples Directory**
   - Check `examples/` for real-world usage patterns
   - Understand how users configure MCP servers

3. **Agent vs Interactive Mode**
   - Compare `agent.rs` and `interactive.rs`
   - Document the "DMN mode" autonomous execution flow

## Questions from MCP Exploration

- How does the agent decide when to call tools vs respond directly?
- What happens if an MCP server crashes mid-session?
- Is there any tool result caching?

## Remaining Files
- `src/display.rs` - Output formatting
- `src/provider.rs` - Provider detection logic
- `dmn_instructions.md` - System prompt for autonomous mode

## Concerns to Investigate
- `client.rs` complexity (706 lines in single file)
- Provider-specific logic scattered vs centralized?
