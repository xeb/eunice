# Eunice v1.0.0 Specification

## Overview

Eunice v1.0.0 is a major simplification of the agentic CLI runner. It strips away multi-agent orchestration, MCP server management, and complex configuration in favor of a minimal, batteries-included design with exactly **4 built-in tools**.

## Design Philosophy

**Sophisticated Simplicity**: One binary, zero configuration, four tools. The agent can discover and learn new capabilities through the Skill system rather than requiring upfront configuration.

**No Built-in System Instructions**: Eunice provides no default system prompt or instructions. The user is responsible for providing context via `--prompt` or during chat. This keeps the tool neutral and flexible.

## CLI Interface

### Flags to Keep

| Flag | Description |
|------|-------------|
| `--model <MODEL>` | AI model to use (auto-detected if not specified) |
| `--prompt <TEXT>` | System prompt (inline or file path) |
| `--chat` | Interactive chat mode with enhanced terminal interface |
| `--webapp` | Start web server interface |
| `--list-models` | List available AI models by provider |
| `--list-tools` | List the 4 built-in tools |
| `--llms-txt` | Output full documentation (includes skill creation guide) |
| `--update` | Update eunice to the latest version |
| `--version` | Show version |
| `--help` | Show help |
| `--verbose` | Enable debug output |
| `--silent` | Suppress non-essential output |

### Flags Removed

- `--shell`, `--filesystem`, `--browser`, `--search`, `--images` (replaced by built-in tools)
- `--all`, `--native` (no longer needed)
- `--config` (no external configuration)
- `--dmn`, `--research` (no multi-agent modes)
- `--agent`, `--list-agents` (no multi-agent architecture)
- `--no-mcp`, `--list-mcp-servers` (no MCP)
- `--api-keys` (simplify to env vars only)
- `--events` (removed JSON-RPC event output)
- `--tool-output-limit` (use sensible defaults)

### Basic Usage

```bash
# Interactive chat (default when no prompt given)
eunice

# Single-shot prompt
eunice "Create a Python script that downloads a URL"

# With explicit model
eunice --model sonnet "Explain this codebase"

# Web interface
eunice --webapp
```

## Built-in Tools

Eunice v1.0.0 automatically registers exactly 4 tools. No configuration required.

### 1. Bash

Execute shell commands with full access to the system.

```json
{
  "name": "Bash",
  "description": "Execute a shell command and return the output.",
  "parameters": {
    "type": "object",
    "properties": {
      "command": {
        "type": "string",
        "description": "The shell command to execute"
      },
      "timeout": {
        "type": "integer",
        "description": "Timeout in seconds (default: 600)"
      }
    },
    "required": ["command"]
  }
}
```

**Implementation**: Uses the existing `ShellServer` from mcpz, executing commands via the user's default shell.

### 2. Read

Read file contents from the filesystem.

```json
{
  "name": "Read",
  "description": "Read the contents of a file.",
  "parameters": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Absolute or relative path to the file"
      }
    },
    "required": ["path"]
  }
}
```

**Implementation**: Direct file read, no directory restrictions. Returns file content as string. For binary files, returns a message indicating the file type.

### 3. Write

Write content to a file.

```json
{
  "name": "Write",
  "description": "Write content to a file. Creates parent directories if needed.",
  "parameters": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Absolute or relative path to the file"
      },
      "content": {
        "type": "string",
        "description": "Content to write to the file"
      }
    },
    "required": ["path", "content"]
  }
}
```

**Implementation**: Creates parent directories automatically. Overwrites existing files.

### 4. Skill

Discover skills from the user's skill library.

```json
{
  "name": "Skill",
  "description": "Search for skills in ~/.eunice/skills/ that can help with a task. Returns matching skill directories with their descriptions from SKILL.md.",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Description of the capability or task you need help with"
      }
    },
    "required": ["query"]
  }
}
```

**Implementation**: See [Skill System](#skill-system) below. Uses a separate LLM call to match the query against skill descriptions.

## Skill System

Skills are user-defined capabilities stored in `~/.eunice/skills/`. Each skill is a directory containing:

```
~/.eunice/skills/
├── image_understanding/
│   ├── SKILL.md           # Required: Instructions and description
│   ├── analyze_image.sh   # Optional: Helper scripts
│   └── examples/          # Optional: Example files
├── web_scraping/
│   ├── SKILL.md
│   └── scrape.py
└── code_review/
    └── SKILL.md
```

### SKILL.md Format

The `## Description` section is **required** and serves as the skill's summary for discovery.

```markdown
# Image Understanding

## Description
Analyze images using vision models. Supports PNG, JPEG, WebP, and PDF files.
Useful for describing screenshots, extracting text from images, or analyzing diagrams.

## Requirements
- GEMINI_API_KEY or OPENAI_API_KEY for vision models
- imagemagick for preprocessing (optional)

## Usage
1. Use the analyze_image.sh script: `./analyze_image.sh <path> "<prompt>"`
2. Or call the Gemini API directly with base64-encoded images

## Available Scripts
- `analyze_image.sh`: Analyze a single image with a custom prompt
```

### Skill Discovery Process

When the agent calls `Skill(query="analyze images")`:

1. **List Skills**: Enumerate all directories in `~/.eunice/skills/`

2. **Extract Descriptions**: For each skill, read the SKILL.md and extract the `## Description` section (required)

3. **LLM Matching**: Spawn a separate LLM call (same model, new context) with:
   ```
   Given these available skills:

   1. image_understanding: Analyze images using vision models. Supports PNG, JPEG, WebP, and PDF files. Useful for describing screenshots, extracting text from images, or analyzing diagrams.

   2. web_scraping: Scrape web pages and extract structured data. Handles JavaScript-rendered pages.

   3. code_review: Review code for bugs, security issues, and style improvements.

   The user is looking for: "analyze images"

   Return the skill names that best match this query, ranked by relevance.
   Return ONLY the skill names, one per line.
   ```

4. **Return Results**: Return the matching skill directories with their descriptions:
   ```
   Found 1 matching skill:

   ## image_understanding
   Path: ~/.eunice/skills/image_understanding/
   Description: Analyze images using vision models. Supports PNG, JPEG, WebP, and PDF files. Useful for describing screenshots, extracting text from images, or analyzing diagrams.

   Read the SKILL.md file for detailed instructions.
   ```

5. **Agent Action**: The main agent can then use `Read` to get the full SKILL.md and `Bash` to execute any helper scripts.

### Default Skills (Auto-Install)

Eunice ships with default skills in the `skills/` directory of the repository. On first run (or when `~/.eunice/skills/` doesn't exist), eunice automatically copies these to the user's skill directory.

```
# Repository structure
eunice/
├── src/
├── skills/                    # Ships with eunice
│   ├── image_analysis/
│   │   └── SKILL.md
│   ├── web_search/
│   │   └── SKILL.md
│   └── git_helper/
│       └── SKILL.md
└── ...

# Auto-installed to
~/.eunice/skills/
├── image_analysis/
├── web_search/
└── git_helper/
```

**Implementation:**
```rust
fn ensure_default_skills() -> Result<()> {
    let skills_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("No home directory"))?
        .join(".eunice/skills");

    if !skills_dir.exists() {
        // Copy embedded skills from binary
        // Skills are embedded via include_dir! or similar
        fs::create_dir_all(&skills_dir)?;
        for (name, content) in EMBEDDED_SKILLS {
            let skill_dir = skills_dir.join(name);
            fs::create_dir_all(&skill_dir)?;
            fs::write(skill_dir.join("SKILL.md"), content)?;
        }
    }
    Ok(())
}
```

Users can delete, modify, or add to `~/.eunice/skills/` as they wish. The auto-install only runs if the directory doesn't exist.

### Why This Design?

- **Zero config**: Skills are just directories with markdown files
- **Composable**: Skills can call other tools (Bash, Read, Write)
- **Discoverable**: Agent can find relevant skills without knowing them upfront
- **User-extensible**: Users add skills by creating directories
- **Version controllable**: Skills are just files, can be git-managed
- **No summarization overhead**: Required `## Description` section eliminates need to extract/summarize
- **Batteries included**: Default skills auto-install on first run

## Model Tool Support Detection

Not all models support function calling. Eunice must detect this and fall back gracefully.

### Detection Strategy

```rust
/// Check if a model supports tool/function calling
pub fn supports_tools(provider: &Provider, model: &str) -> bool {
    match provider {
        // All modern OpenAI models support tools
        Provider::OpenAI => true,

        // All Gemini models we support have tool capability
        Provider::Gemini => true,

        // All Claude models support tools
        Provider::Anthropic => true,

        // Ollama: check by model family
        Provider::Ollama => {
            let model_lower = model.to_lowercase();

            // Known tool-supporting model families
            let tool_families = [
                "llama3.1", "llama3.2", "llama3.3",
                "qwen2", "qwen2.5", "qwen3",
                "mistral-nemo", "mistral-large",
                "command-r",
                "granite",
                "hermes",
            ];

            tool_families.iter().any(|f| model_lower.contains(f))
        }
    }
}
```

### Fallback Behavior

When a model doesn't support tools:

1. **Warn the user**: Display a message at startup
   ```
   Warning: Model 'gemma3:latest' does not support function calling.
   Running in text-only mode (no Bash/Read/Write/Skill tools available).
   ```

2. **Disable tools**: Run as a simple chat interface without tool capabilities

3. **Suggest alternatives**:
   ```
   Tip: For full tool support, try: llama3.1, qwen2.5, or mistral-nemo
   ```

### Runtime Validation (Optional Enhancement)

For unknown Ollama models, attempt a minimal tool call during initialization:

```rust
async fn test_tool_support(client: &Client, model: &str) -> bool {
    let test_tool = Tool {
        name: "test".into(),
        description: "Test tool".into(),
        parameters: json!({"type": "object", "properties": {}}),
    };

    match client.chat_with_tools(model, "Say hi", vec![test_tool]).await {
        Ok(_) => true,
        Err(e) if e.to_string().contains("tools") => false,
        Err(_) => true, // Other errors don't indicate tool problems
    }
}
```

## Features to Keep

### Output Truncation

Keep the `OutputStore` system for truncating large tool outputs:
- First 50 + last 50 lines shown to LLM
- Full output stored in memory/temp files
- `get_output` tool available for retrieving middle sections

### Context Compaction

Keep the compaction system for long conversations:
- Lightweight compaction: Truncate old tool outputs
- Full summarization: LLM-generated summary when context exhausted
- Automatic trigger on RESOURCE_EXHAUSTED errors

### Chat Interface

Keep all TUI/interactive mode features:
- Bracketed paste support
- Escape/Ctrl+C cancellation
- Command menu
- Readline support

### Webapp Mode

Keep the webapp with:
- SSE streaming
- Session management
- SQLite persistence (when available)
- Mobile responsive design

## Architecture Changes

### Removed Components

| Component | Replacement |
|-----------|-------------|
| `src/mcp/` | Built-in tools (no MCP subprocess management) |
| `src/orchestrator/` | Removed (no multi-agent) |
| `src/config.rs` (MCP loading) | Removed |
| `McpManager` | `BuiltinToolRegistry` with 4 tools |
| `AgentConfig`, multi-agent types | Removed |
| DMN/Research modes | Removed |
| `dmn_instructions.md` | Removed (no built-in system instructions) |
| `prompts/` directory | Removed (user provides all prompts) |

### Simplified File Structure

```
src/
├── main.rs              # CLI entry, simplified arg parsing
├── models.rs            # Core types (Message, Tool, Provider)
├── client.rs            # HTTP client for AI providers
├── provider.rs          # Provider detection + tool support check
├── agent.rs             # Single agent loop
├── tools/               # NEW: Built-in tools
│   ├── mod.rs
│   ├── bash.rs          # Shell execution
│   ├── read.rs          # File reading
│   ├── write.rs         # File writing
│   └── skill.rs         # Skill discovery
├── skills.rs            # NEW: Skill system (discovery, SKILL.md parsing)
├── output_store.rs      # Output truncation (keep)
├── compact.rs           # Context compaction (keep)
├── display.rs           # Terminal UI output (keep)
├── display_sink.rs      # Display abstraction (keep)
├── interactive.rs       # Basic REPL (keep)
├── tui/                 # Enhanced TUI (keep)
├── webapp/              # Web interface (keep)
└── usage.rs             # Token tracking (keep)
```

### Removed Binaries

- `mcpz` binary: No longer needed (MCP removed)
- `browser` binary: No longer needed (browser automation removed)

Only the `eunice` binary remains.

## Migration Path

### For Users

1. **Skills Migration**: Move any custom MCP servers to skill directories
   - Create `~/.eunice/skills/<name>/SKILL.md`
   - Add shell scripts that wrap the functionality

2. **Config Migration**: `eunice.toml` no longer used
   - Move system prompts to `--prompt` flag or prompt files
   - Move API keys to environment variables

### For Existing Workflows

| Old Usage | New Usage |
|-----------|-----------|
| `eunice --dmn "task"` | `eunice --prompt dmn.md "task"` (provide your own instructions) |
| `eunice --shell --filesystem` | `eunice` (tools auto-enabled) |
| `eunice --config myconfig.toml` | Create skills in `~/.eunice/skills/` |
| `eunice --agent worker "task"` | Use single agent with Skill tool |

**Note on DMN mode**: The v0.x DMN instructions are no longer built-in. If you relied on DMN behavior, save the instructions to a file and use `--prompt`.

## Version Comparison

| Feature | v0.3.x | v1.0.0 |
|---------|--------|--------|
| Multi-agent | ✅ | ❌ |
| MCP servers | ✅ | ❌ |
| Config files | ✅ | ❌ |
| Built-in system instructions | ✅ (DMN) | ❌ |
| Built-in tools | 0-5 (flags) | 4 (always) |
| Skill discovery | ❌ | ✅ |
| Default skills | ❌ | ✅ (auto-install) |
| Tool support detection | ❌ | ✅ |
| Binaries | 3 (eunice, mcpz, browser) | 1 (eunice) |
| Lines of code | ~8000 | ~4000 (est.) |

## Managing Skills

### Installing Skills Manually

To add a skill from a git repository:

```bash
# Clone into skills directory
git clone https://github.com/user/my-skill ~/.eunice/skills/my-skill

# Or copy a directory
cp -r /path/to/skill ~/.eunice/skills/
```

### Creating Your Own Skills

1. Create a directory in `~/.eunice/skills/`:
   ```bash
   mkdir ~/.eunice/skills/my_skill
   ```

2. Create `SKILL.md` with required `## Description`:
   ```markdown
   # My Skill

   ## Description
   One paragraph describing what this skill does and when to use it.

   ## Usage
   Instructions for the agent...
   ```

3. Optionally add helper scripts:
   ```bash
   # ~/.eunice/skills/my_skill/run.sh
   #!/bin/bash
   # Your skill logic here
   ```

### Skill Documentation in --llms-txt

The `eunice --llms-txt` output includes a guide on creating skills, which can be fed to other LLMs or used as reference.

## Implementation Plan

### Phase 1: Core Simplification
1. Remove MCP, orchestrator, multi-agent code
2. Implement 4 built-in tools directly
3. Add tool support detection
4. Update CLI flags

### Phase 2: Skill System
1. Implement SKILL.md parsing
2. Implement skill discovery with LLM matching
3. Create example skills for testing

### Phase 3: Cleanup
1. Remove mcpz and browser binaries from build
2. Update documentation
3. Update tests
4. Release v1.0.0

---

*Spec Version: 1.0*
*Last Updated: 2026-02-07*
