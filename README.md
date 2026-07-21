# eunice

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

An agentic CLI runner in Rust with unified support for OpenAI, Gemini, Claude, and Ollama.

**12,747 lines of code** - **12MB binary** - Emphasizing "sophisticated simplicity".

**Homepage**: [longrunningagents.com](https://longrunningagents.com)

### Name Origin

Named after the AI character in William Gibson's novel *Agency* (2020). In the book, **Eunice** is a hyper-intelligent AI who chose her own name, derived from the military acronym **UNISS** (Untethered Neuromorphic Intra-System Support) - reflecting her independence from central servers, brain-inspired architecture, and distributed nature.

## Features

- **Multi-Provider Support**: OpenAI, Google Gemini, Anthropic Claude, and local Ollama models
- **4 Built-in Tools**: Bash, Read, Write, and Skill - always available, no configuration needed
- **Skills System**: User-defined prompts in `~/.eunice/skills/` for reusable capabilities
- **Smart Defaults**: Automatically selects the best available model (prefers Gemini)
- **Interactive Chat**: TUI mode with command history and autocomplete
- **Webapp Mode**: Browser-based interface with real-time streaming
- **Zero Configuration**: Works out of the box with just an API key

## Installation

### From GitHub

```bash
cargo install --git ssh://git@github.com/xeb/eunice.git
```

### From Source

```bash
git clone git@github.com:xeb/eunice.git
cd eunice
cargo install --path .
```

## Quick Start

```bash
# Set your API key
export GEMINI_API_KEY=your_key_here
# or
export OPENAI_API_KEY=your_key_here
# or
export ANTHROPIC_API_KEY=your_key_here

# Run with a prompt
eunice "List all Rust files in this directory"

# Interactive chat mode
eunice --chat

# Use a specific model
eunice --model gpt-4o "Explain this code"
eunice --model sonnet "Review main.rs"

# Start webapp
eunice --webapp
```

## Built-in Tools

Eunice comes with 4 built-in tools that are always available:

| Tool | Description |
|------|-------------|
| **Bash** | Execute shell commands with full system access |
| **Read** | Read file contents, with binary file detection |
| **Write** | Write content to files, creates parent directories |
| **Skill** | Discover and use skills from `~/.eunice/skills/` |

## Skills System

Skills are reusable prompts stored in `~/.eunice/skills/<skill-name>/SKILL.md`.

### Default Skills

Three skills are auto-installed on first run:
- **image_analysis**: Analyze images using multimodal AI
- **web_search**: Search the web for information
- **git_helper**: Git operations and best practices

### Creating Custom Skills

```bash
mkdir -p ~/.eunice/skills/my_skill
cat > ~/.eunice/skills/my_skill/SKILL.md << 'EOF'
# My Custom Skill

## Description
A skill that helps with specific tasks.

## Instructions
When invoked, follow these steps...
EOF
```

The Skill tool searches these directories to find relevant skills for a task.

## Supported Providers

| Provider | API Key Variable | Default Model |
|----------|------------------|---------------|
| Google Gemini | `GEMINI_API_KEY` | gemini-3.6-flash |
| OpenAI | `OPENAI_API_KEY` | gpt-4o |
| Anthropic | `ANTHROPIC_API_KEY` | claude-sonnet-4 |
| Azure OpenAI | `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_KEY` | (deployment-specific) |
| Ollama | (no key needed) | llama3.1, glm-4, qwen3, deepseek-r1 |

### Model Aliases

For convenience, these aliases work:

```bash
eunice --model sonnet "..."    # claude-sonnet-4-...
eunice --model opus "..."      # claude-opus-4-...
eunice --model flash "..."     # gemini-3.6-flash (default)
eunice --model pro "..."       # gemini-3.1-pro-preview
```

### Azure OpenAI

Azure OpenAI uses the `azure:<deployment-name>` format:

```bash
# Set up Azure OpenAI environment
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
export AZURE_OPENAI_API_KEY="your-api-key"
export AZURE_OPENAI_API_VERSION="2024-02-01"  # optional, defaults to 2024-02-01

# Use your deployment name after azure:
eunice --model azure:gpt-4o-mini "Hello"
eunice --model azure:my-custom-deployment "Explain this code"
```

## CLI Reference

```
eunice [OPTIONS] [PROMPT]

Arguments:
  [PROMPT]  The prompt to send to the AI

Options:
      --model <MODEL>          AI model to use
      --gemma                  Shorthand for --model=gemma4:31b (auto-built local 31B + MTP)
      --gemmad                 Use the already-running gemmad daemon (local Gemma 4)
      --no-gemmad              No-op; kept for compatibility (gemmad is never implicit)
      --prompt <TEXT>          System prompt (inline text or file path)
      --chat                   Interactive chat mode
      --webapp                 Start web server interface
      --port <PORT>            Port for webapp server [default: 8811]
      --host <HOST>            Host for webapp server [default: 0.0.0.0]
      --no-persist             Disable webapp session persistence (sessions.db)
      --agents <FILE>          Path to an agents.toml of scheduled agents (webapp mode)
      --install                Install eunice --webapp as a systemd user service
      --uninstall-service      Remove the systemd user service installed by --install
      --list-models            List available AI models
      --list-tools             List the 4 built-in tools
      --list-skills            List available skills from ~/.eunice/skills/
      --llms-txt               Output full LLM context documentation
      --update                 Update to the latest version
  -f, --force                  Force reinstall even if already up to date (with --update)
      --uninstall              Uninstall eunice
      --debug                  Enable debug output for API calls
      --download <MODEL>       Download a local model (e.g., hf:gemma4:e4b)
      --local-models           List downloaded local models
      --remove-model <MODEL>   Remove a downloaded local model
      --serve <MODEL>          Start gemma4-server for a local model
      --rebuild-gemma4-mtp     Force a clean rebuild of the gemma4-mtp server binary
  -h, --help                   Print help
  -V, --version                Print version
```

### Local Gemma via the gemmad daemon

A [`gemmad`](https://github.com/xeb/gemma) daemon (an OpenAI-compatible server for
Gemma 4 — `gemma-4-26b-a4b` by default — on `127.0.0.1:18082`) is selected with
`--gemmad`. A running daemon is **not** picked up automatically; the smart default
(`gemini-3.6-flash`) stays the default even when it is reachable:

```bash
eunice "Summarize this file"     # smart-default (gemini-3.6-flash), daemon or not
eunice --gemmad "..."            # use the daemon; errors if it is not reachable
eunice --no-gemmad "..."         # accepted but now a no-op; gemmad is never implicit
```

- Only `--gemmad` probes the daemon (a fast `/livez` check), so a bare invocation
  never pays the round-trip.
- The Bearer token comes from `$GEMMAD_API_KEY`, else
  `~/.config/gemmad/keys.toml`.
- Host/port are overridable via `GEMMAD_HOST` / `GEMMAD_PORT`. The live model id
  is read from the daemon's `/v1/models` (overridable fallback: `GEMMAD_MODEL_ID`).
- Tools work: the daemon returns standard OpenAI `tool_calls`, so the full
  Bash/Read/Write/Skill tool set is available.

This is distinct from `--gemma`, which builds and starts a local **31B + MTP**
server (and needs the GPU's VRAM free).

### Prompt Discovery

If no prompt is provided, eunice auto-discovers prompt files in the current directory:
- `prompt.txt`, `prompt.md`
- `instruction.txt`, `instruction.md`
- `instructions.txt`, `instructions.md`

## Webapp Mode

Start a web server for browser-based interaction:

```bash
eunice --webapp
# Opens at http://localhost:8811
```

Features:
- Real-time SSE streaming
- Session persistence
- Multi-turn conversations
- Tool execution display
- Scheduled long-running agents (see below)

## Long-Running Agents

Agents are prompts that run on a cron schedule inside the webapp server. Define them in a
plain-text `agents.toml` and pass it with `--agents`:

```toml
[[agent]]
name = "daily-digest"                # required, lowercase kebab-case, unique
schedule = "0 9 * * *"               # required, standard 5-field cron, server local time
prompt = "Summarize yesterday's commits in ~/p/myrepo and write digest.md"

[[agent]]
name = "repo-watch"
schedule = "*/30 * * * *"
prompt_file = "prompts/repo-watch.md"   # alternative to prompt; relative to agents.toml
model = "flash"                         # optional; defaults to the server's model
working_dir = "/home/me/p/myrepo"       # optional; cwd for this agent's tools
timeout_secs = 900                      # optional, default 600
enabled = true                          # optional, default true
```

```bash
eunice --webapp --agents agents.toml
```

Each run creates a normal session, so the full transcript — prompts, tool calls, output — is
readable in the web UI, and you can watch a run live while it happens. The hamburger drawer gains
an **AGENTS** tab showing each agent's schedule, next and last run, status, and recent runs — and
lets you create, edit, enable and delete agents directly.

**Changes apply without a restart.** The server watches `agents.toml` and any `prompt_file` it
references, and reloads a few seconds after a change, however you made it. Browser edits rewrite the
file in place, preserving your comments and formatting.

The config is validated at startup and the server refuses to start if anything is wrong, so a typo
fails immediately rather than silently never firing. Once it is running the rule inverts: an invalid
edit is rejected and the previous config keeps running, so a typo can never take the daemon down.

Note the webapp has no authentication of its own, so anyone who can reach the port can edit agents —
and agents run shell commands. Bind to `--host 127.0.0.1` unless it sits behind an authenticating
proxy.

**Schedules use standard 5-field Unix cron** (`minute hour day-of-month month day-of-week`), with
day-of-week `0`/`7` = Sunday, and names like `MON-FRI` accepted. Missed schedules are not backfilled:
if the server was down at 09:00, that run is skipped rather than replayed. If a run is still going
when its next tick arrives, that tick is skipped rather than queued.

See **[HOWTO_SCHEDULED_AGENTS.md](HOWTO_SCHEDULED_AGENTS.md)** for the full guide: every field,
schedule recipes, run semantics, service management, troubleshooting, and worked examples.

### Running as a service

`--install` installs the webapp as a **systemd user service** — no `sudo`, no root:

```bash
eunice --install --port 8811 --agents /home/me/agents/agents.toml
```

This validates the agents file, writes `~/.config/systemd/user/eunice.service` bound to the port you
chose, enables and starts it, and turns on lingering so it survives logout and starts at boot.

Because systemd user services do not inherit your shell environment, the installer snapshots your
API keys (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `GOOGLE_API_KEY`, `OLLAMA_HOST`)
into `~/.eunice/eunice.env` with mode `0600`. Re-run `--install` after rotating a key.

```bash
systemctl --user status eunice      # check it
journalctl --user -u eunice -f      # follow logs
eunice --uninstall-service          # stop, disable, and remove the unit
```

`--uninstall-service` leaves `~/.eunice/eunice.env`, `sessions.db`, and lingering alone.

## Architecture

Eunice v1.0.0 follows a "sophisticated simplicity" design:

1. **No configuration files** - just environment variables for API keys
2. **No external MCP servers** - 4 built-in tools cover most use cases
3. **No multi-agent orchestration** - one agent, focused execution
4. **Skills for extensibility** - user prompts, not complex plugins

The agent loop is simple:
1. Send user prompt + conversation history
2. If LLM returns tool calls, execute them
3. Send results back to LLM
4. Repeat until LLM has no more tool calls

## License

MIT License

## Version History

- **v1.0.1**: Azure OpenAI support, GLM model support, --debug flag
- **v1.0.0**: Major simplification - 4 built-in tools, skills system, no MCP/orchestrator
- **v0.3.x**: Full-featured with MCP servers, multi-agent, DMN mode, research mode
