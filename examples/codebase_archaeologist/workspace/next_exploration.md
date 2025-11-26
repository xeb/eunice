# Next Steps

## Priority Explorations

1. **Deep Dive: client.rs** (High Priority)
   - 706 lines - largest file in the codebase
   - Understand OpenAI vs Gemini Native API normalization
   - Map streaming/non-streaming code paths
   - Document the response conversion logic

2. **Entry Point: main.rs**
   - Command-line argument handling (clap)
   - Mode selection logic
   - How all components are wired together

3. **Examples Directory**
   - Check `examples/` for real-world usage patterns
   - Document expected configuration file format (`eunice.json`)
   - See practical usage of DMN and MCP tools

## Completed Explorations

| Component | Date | Status |
|-----------|------|--------|
| Core (src) | 2025-11-26 | ✅ Overview |
| Provider/Client | 2025-11-25 | ✅ Patterns documented |
| MCP Layer | 2025-11-26 | ✅ Full analysis |
| DMN/Agent | 2025-11-26 | ✅ Full analysis |
| Display | 2025-11-26 | ✅ Full analysis |

## Questions from Exploration

### From DMN/Agent Exploration (2025-11-26)
- How does the agent handle context window exhaustion?
- What happens if an MCP server crashes mid-agent-loop?
- Is there any token counting or context management?
- How would you extend the hardcoded DMN server list?

### From MCP Exploration (2025-11-26)
- How does the agent decide when to call tools vs respond directly?
- What happens if an MCP server crashes mid-session?
- Is there any tool result caching?

## Remaining Files

| File | Lines | Status |
|------|-------|--------|
| `src/client.rs` | 706 | Partially covered |
| `src/main.rs` | 175 | Referenced but not deep-dived |
| `src/lib.rs` | 5 | Module declarations only |
| `examples/` | ? | Not explored |

## Concerns to Investigate

1. **client.rs complexity** - 706 lines in single file; may benefit from splitting
2. **Hardcoded model lists** - Need maintenance strategy
3. **No context/token management** - Could hit limits on long tasks
4. **MCP server version pinning** - Currently fetches latest, risky
