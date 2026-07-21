# Eunice v1.0.0 - Development Guide

## About

Eunice is an agentic CLI runner written in Rust that provides a unified interface for multiple AI providers (OpenAI, Gemini, Anthropic Claude, and Ollama). It emphasizes "sophisticated simplicity" - minimal configuration with maximum capability.

## Architecture

### Provider System

The codebase uses a provider abstraction layer that routes requests to different AI APIs:

```
User Input -> Provider Detection -> Client -> API Request -> Response
```

**Special Case: Gemini API Dual Support**
- Gemini 1.x/2.x models: Use OpenAI-compatible API (`/v1beta/openai/`)
- Gemini 3.x models (`gemini-3*`, e.g. `gemini-3.6-flash` (the default), `gemini-3.1-pro-preview`): Use native Gemini API (`/v1beta/models/{model}:generateContent`) — required because Gemini 3 thought signatures must be round-tripped on function calls

### Key Components

1. **Provider Detection** (`src/provider.rs`)
   - Detects model -> provider mapping
   - Handles model aliases (e.g., `sonnet` -> `claude-sonnet-4-...`)
   - Sets `use_native_gemini_api` flag for special models
   - `supports_tools()` function for Ollama model capability detection

2. **Client** (`src/client.rs`)
   - HTTP client with provider-specific headers
   - Message format conversion (OpenAI <-> Gemini)
   - Retry logic for transient failures

3. **Tools** (`src/tools/`)
   - `ToolRegistry` - centralized registry for 4 built-in tools
   - `BashTool` - shell command execution
   - `ReadTool` - file reading with binary detection
   - `WriteTool` - file writing with directory creation
   - `SkillTool` - skill discovery and search

4. **Skills** (`src/skills.rs`)
   - `~/.eunice/skills/<name>/SKILL.md` format
   - `ensure_default_skills()` - auto-install on first run
   - `discover_skills()` - keyword-based skill matching

5. **Agent Loop** (`src/agent.rs`)
   - Main conversation loop
   - Tool execution with output store
   - Conversation history management
   - Built-in `get_output` tool for large output retrieval

6. **Output Store** (`src/output_store.rs`)
   - Stores full tool outputs in memory
   - Truncates output to first 50 + last 50 lines for LLM context
   - Provides `get_output` tool for retrieving middle sections

7. **TUI Mode** (`src/tui/app.rs`)
   - Uses `r3bl_tui` library for enhanced terminal interface
   - Command menu, readline support, cancel with Escape

8. **Interactive Mode** (`src/interactive.rs`)
   - Simpler REPL loop with basic readline
   - Fallback when TUI cannot initialize

9. **Webapp Mode** (`src/webapp/`)
   - Axum web server with SSE streaming
   - Session persistence: SQLite `sessions.db` in cwd by default; `--no-persist` for in-memory
   - System prompt via `--prompt <file>` (or auto-discovered `prompt.md`), prepended to each new session
   - Multi-turn conversations
   - The agent loop for webapp mode is `handlers::run_agent_with_events`, **not** `agent::run_agent`

10. **Scheduled Agents** (`src/agents.rs`, `src/webapp/scheduler.rs`)
   - `agents.toml` (via `--agents`) declares agents with a cron `schedule` and a prompt
   - Config parsing/validation lives in `src/agents.rs` (lib tree, no webapp deps); the runtime
     loop lives in `src/webapp/scheduler.rs`
   - Validated at startup — the server refuses to start on a bad config
   - Each run drives `run_agent_with_events` into a session tagged with `sessions.agent_name`,
     so runs are viewable (and watchable live) in the normal web UI
   - **Cron dialect:** `agents.toml` takes standard 5-field Unix cron. The `cron` crate needs
     6 fields (seconds first) and numbers day-of-week 1=Sunday..7=Saturday, so
     `agents::normalize_cron` translates. Do not pass a raw user expression to `cron::Schedule`.
   - **Day fields intersect, they do not union.** `0 9 1 * 1` fires only on a Monday that is
     also the 1st; Unix cron would fire on either. `agents::restricts_both_day_fields` detects
     this, and both the startup log and the web editor warn about it.
   - **Requires `cron` >= 0.17.** 0.12 called chrono's panicking `ymd` and aborted on an
     ambiguous local *date* — real in midnight-transition zones (Chile, Cuba, Lebanon) — which
     silently killed the scheduler task while the web UI kept serving. 0.12 also skipped fires
     landing in a DST fall-back repeated hour, dropping real runs (`0 2 * * *` skipped Nov 1
     each year in US zones). Do not downgrade.
   - **Only ever call `.next()` on a `Schedule`.** 0.17's `after()` is non-monotonic inside a
     fall-back overlap: it interleaves the two passes of the repeated hour and can emit times
     that do not match the expression. Taking just the first element is safe and is what every
     call site does.
   - **Known gap:** chrono cannot parse `Africa/Casablanca` and silently falls back to another
     zone (measured -07:00 where +01:00 is correct). A server there would schedule wrongly and
     disagree with the browser preview, which uses `Intl` and is correct. Fixing it properly
     means moving off `chrono::Local` to `chrono-tz`.

11. **Daemon Install** (`src/daemon.rs`)
   - `--install` writes a systemd **user** unit (no sudo), enables lingering, and snapshots API
     keys into `~/.eunice/eunice.env` (mode 0600) because user services do not inherit the
     login shell environment
   - Bin-only (`mod daemon;` in main.rs, absent from lib.rs)

> **Dual-target trap:** `src/lib.rs` and `src/main.rs` declare *separate* module trees, and
> `webapp`/`tui` exist only in main.rs's. A module added to lib.rs must not reference
> `crate::webapp::*`.

## Testing

The project includes **328 unit tests** covering:
- Provider detection logic
- Message format conversions
- Response parsing
- Tool execution
- Session persistence, schema migration, and agent-run status
- `agents.toml` parsing, validation, and cron translation
- Scheduler run-state transitions and API serialization
- systemd unit rendering

One test is `#[ignore]`d by design: the DST regression test re-execs the test binary
with `TZ=America/Santiago` and drives the ignored child, because chrono *caches* the
local zone — setting `TZ` in-process would be order-dependent and would corrupt
`Local` for every other test in the binary.

Run tests with:
```bash
cargo test
```

## File Structure

```
src/
├── main.rs              - CLI entry, arg parsing
├── lib.rs               - Library exports
├── models.rs            - Data structures
├── client.rs            - HTTP client, format conversions
├── provider.rs          - Provider detection
├── agent.rs             - Agent loop with tool execution
├── agents.rs            - agents.toml parsing, validation, cron translation
├── daemon.rs            - systemd user-service install/uninstall (bin-only)
├── tools/
│   ├── mod.rs           - ToolRegistry (optional per-agent working dir)
│   ├── bash.rs          - Bash tool
│   ├── read.rs          - Read tool
│   ├── write.rs         - Write tool
│   └── skill.rs         - Skill tool
├── skills.rs            - Skill system
├── display.rs           - Terminal UI output
├── display_sink.rs      - Display abstraction (stdout/TUI)
├── interactive.rs       - Interactive REPL mode
├── compact.rs           - Context compaction
├── output_store.rs      - Large output storage
├── usage.rs             - Token usage tracking
├── tui/
│   ├── mod.rs
│   └── app.rs           - TUI mode with r3bl_tui
└── webapp/
    ├── mod.rs
    ├── server.rs        - Axum web server
    ├── handlers.rs      - HTTP/SSE handlers + the webapp agent loop
    ├── scheduler.rs     - Cron loop, agent registry, /api/agents
    └── persistence.rs   - Session storage

skills/
├── image_analysis/SKILL.md
├── web_search/SKILL.md
└── git_helper/SKILL.md
```

## Line Count and Binary Size

When updating the codebase, update both metrics in README.md:

### Count Implementation Lines
```bash
total=0
for file in src/*.rs src/tools/*.rs src/webapp/*.rs src/tui/*.rs; do
  test -f "$file" || continue
  test_start=$(grep -n "^#\[cfg(test)\]" "$file" 2>/dev/null | cut -d: -f1 | head -1)
  if [ -n "$test_start" ]; then
    lines=$((test_start - 1))
  else
    lines=$(wc -l < "$file")
  fi
  total=$((total + lines))
done
echo "Total: $total lines"
```

### Check Binary Size
```bash
cargo build --release && ls -lh target/release/eunice
```

### Version String

The version includes a 5-character git hash, embedded at compile time via `build.rs`:

```
$ eunice --version
eunice 1.0.0 (f434f)
```

The hash is captured using:
```rust
// build.rs
println!("cargo:rustc-env=GIT_HASH={}", git_hash);

// main.rs
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"))]
```

## Development Workflow

### Adding a New Provider

1. Update `Provider` enum in `src/models.rs`
2. Add detection logic in `src/provider.rs::detect_provider()`
3. Handle authentication in `src/client.rs::new()`
4. Add `supports_tools()` logic if needed
5. Add tests

### Adding a New Built-in Tool

1. Create `src/tools/newtool.rs` with struct and `get_spec()`/`execute()` methods
2. Add to `ToolRegistry` in `src/tools/mod.rs`
3. Update `--list-tools` output
4. Add tests

### Creating a Skill

1. Create `skills/skill_name/SKILL.md` with `## Description` section
2. Add to `DEFAULT_SKILLS` in `src/skills.rs` with `include_str!`

## Dependencies

- **tokio**: Async runtime
- **reqwest**: HTTP client
- **serde/serde_json**: Serialization
- **clap**: CLI with aliases
- **colored**: Terminal colors
- **crossterm**: Terminal control
- **r3bl_tui**: TUI readline
- **axum**: Web server
- **anyhow**: Error handling

## Version History

- **v1.0.0**: Major simplification
  - 4 built-in tools (Bash, Read, Write, Skill)
  - Skills system for extensibility
  - Removed MCP servers
  - Removed multi-agent orchestration
  - Removed DMN mode
  - Removed browser/mcpz binaries
  - Single binary: eunice
  - Native Gemini API streaming support
  - Parallel function calling (grouped tool responses)
  - Animated thinking spinner (purple/magenta)
  - Git hash in version string (5 characters)

## Releasing

When releasing:
1. Run `cargo test` - all tests must pass
2. Update LOC and binary size in README.md
3. Bump version in Cargo.toml
4. Git commit with descriptive message
5. Git push
6. Update `~/p/longrunningagents.com/version.txt` to the new version and deploy:
   `wrangler pages deploy ~/p/longrunningagents.com --project-name=longrunningagents --commit-dirty=true --branch=master`
   (longrunningagents.com is a Cloudflare Pages project — see its CLAUDE.md; the old
   `~/gal` server is dead. `eunice --update` checks https://longrunningagents.com/version.txt)
7. Run `eunice --update` to verify
