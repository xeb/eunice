# eunice - Agentic CLI Runner

## Project Overview

eunice is a generalist, minimalist agent framework that serves as an agentic CLI runner. It enables natural language interactions with AI models that can execute local file operations through a simple tool-calling interface.

The framework supports multiple AI providers (OpenAI, Google Gemini, and local Ollama models) and provides a unified interface for agent-based task execution with built-in tool capabilities.

## Core Architecture

### Agent Framework
eunice implements an agentic workflow where:
1. User provides a natural language prompt
2. AI model processes the prompt and may decide to call tools
3. Tools execute locally (file operations) and return results
4. AI model processes tool results and continues until task completion
5. The loop continues until the agent determines the task is complete

### Tool System
eunice supports tool integration exclusively through Model Context Protocol (MCP) servers:

#### MCP Server Integration
All tool capabilities are provided through Model Context Protocol (MCP) servers configured via JSON configuration files. MCP servers run as separate processes and communicate via stdio, providing tools like filesystem access, database connections, API integrations, memory management, time operations, web fetching, and more.

**No Built-in Tools**: eunice has no hardcoded tools. If no `--config` is specified, no tools are available to the AI model.

All tools return structured data that the AI models can process and act upon.

### Provider Support
- **OpenAI**: gpt-3.5-turbo, gpt-4, gpt-4o, gpt-4-turbo, gpt-5, chatgpt-4o-latest
- **Google Gemini**: gemini-2.5-flash, gemini-2.5-pro, gemini-1.5-flash, gemini-1.5-pro
- **Anthropic**: claude-sonnet-4-20250514, claude-opus-4-1-20250805, or use aliases: `sonnet`, `opus`, `claude-sonnet`, `claude-opus`
- **Ollama**: Any locally installed model (validated via `ollama list`)

### Smart Default Model Selection
eunice now features intelligent model selection that automatically chooses the best available model:

**Priority Order:**
1. **Ollama models** (if available): `gpt-oss:latest`, `deepseek-r1:latest`, `llama3.1:latest`
2. **Gemini** (if API key exists): `gemini-2.5-flash`
3. **Anthropic** (if API key exists): `sonnet`
4. **OpenAI** (if API key exists): `gpt-4o`
5. **Fallback**: Error with helpful suggestions for installation/configuration

This eliminates the need to specify a model for most use cases - eunice will automatically select the best available option.

## Installation Methods

### Method 1: Global Installation (Recommended)
```bash
uv tool install .
```
Creates a global `eunice` command available system-wide.

### Method 2: Direct Script Execution
```bash
uv run eunice.py "your prompt here"
```
Runs directly with automatic dependency management.

### Method 3: Traditional Python
```bash
pip install openai
python eunice.py "your prompt here"
```

### Troubleshooting Installation

If the `eunice` command isn't working or you're getting an older version without recent features (like `--silent`), you may need to uninstall and reinstall:

```bash
# Uninstall the current version
uv tool uninstall eunice

# Reinstall the latest version
uv tool install .
```

This ensures you have the most recent version with all available features.

## Configuration

### Environment Variables
Required API keys must be set as environment variables:
- `OPENAI_API_KEY` - Required for OpenAI models
- `GEMINI_API_KEY` - Required for Gemini models
- `ANTHROPIC_API_KEY` - Required for Anthropic models
- Ollama models run locally and don't require API keys

### Provider Detection Logic
1. **Anthropic**: Models starting with "claude" or matching aliases (`sonnet`, `opus`) → Anthropic API
2. **Gemini**: Models starting with "gemini" → Gemini API
3. **Ollama**: Check local availability via `ollama list` (highest priority for installed models)
4. **OpenAI**: Models matching patterns (gpt*, chatgpt*, etc.) → OpenAI API
5. **Fallback**: Unknown models default to Ollama with validation

This ensures models like `gpt-oss` (an Ollama model) are correctly routed to Ollama instead of OpenAI, and that Anthropic models are properly handled.

## Command Line Interface

### Basic Usage
```bash
# Smart default model selection (automatically chooses best available)
eunice "How many files are in the current directory?"

# Explicit model specification
eunice --model="gpt-4" "analyze this codebase"
eunice --model="gemini-2.5-pro" --prompt=./analysis_request.txt
eunice --model="sonnet" "explain the code structure"
eunice --model="opus" "review this implementation"

# Configuration and output control
eunice --config=./mcp-config.json "analyze my project structure"
eunice --silent "quiet operation without visual elements"
eunice --verbose "enable debug output to /tmp/eunice_debug.log"
eunice --no-mcp "analyze code without any MCP tools"

# Interactive mode for ongoing conversations
eunice --interact

# With eunice.json in current directory (automatically loaded)
eunice "What time is it and how many files are here?"

# Disable MCP even if eunice.json exists
eunice --no-mcp "simple analysis without tools"

# Empty config functions like --no-mcp
eunice --config='' "no MCP tools available"
```

### Options
- `--model=MODEL` - Specify AI model (smart default: available Ollama → Gemini → Anthropic → OpenAI)
- `--prompt=PROMPT` - Prompt as named argument (can be file path or string)
- `--tool-output-limit=N` - Limit tool output display (default: 50, 0 = no limit)
- `--silent` - Suppress all output except AI responses (hide tool calls, model info, MCP displays)
- `--verbose` - Enable verbose debug output to /tmp/eunice_debug.log
- `--config=CONFIG_FILE` - Path to JSON configuration file for MCP servers
- `--no-mcp` - Disable MCP server loading even if eunice.json exists
- `--sysadmin` - Enable sysadmin mode with auto-loaded MCP tools and system instructions for software engineering tasks
- `--interact` - Start interactive mode for ongoing conversations
- `--list-models` - Show all available models grouped by provider
- `--version` - Show program version number
- `--help` - Enhanced help with model availability and API key status

### Prompt Handling
- Prompts can be provided as positional arguments or via `--prompt`
- Automatic file detection: if prompt looks like a file path and exists, content is read
- Supports both direct strings and file-based prompts

### MCP Server Configuration

eunice supports Model Context Protocol (MCP) servers to extend tool capabilities beyond the built-in file operations.

#### Automatic Configuration Loading
- **Default Behavior**: If a file named `eunice.json` exists in the current directory, it will be automatically loaded as the MCP configuration
- **Manual Override**: Use `--config=path/to/config.json` to specify a different configuration file
- **Disabling MCP**: Use `--no-mcp` to disable MCP server loading even if `eunice.json` exists
- **Empty Config**: Using `--config=''` (empty string) functions the same as `--no-mcp`
- **Priority**: Explicit `--config` parameter takes precedence over automatic `eunice.json` detection
- **Validation**: `--no-mcp` and `--config` cannot be used together and will result in an error

#### Configuration File Format
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "."
      ]
    },
    "memory": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-memory",
        "."
      ]
    },
    "time": {
      "command": "uvx",
      "args": [
        "mcp-server-time"
      ]
    }
  }
}
```

#### MCP Server Lifecycle
1. **Startup**: eunice spawns each configured MCP server as a subprocess
2. **Initialization**: Handshake and capability exchange via stdio
3. **Tool Discovery**: Each MCP server exposes its available tools via the MCP protocol
4. **Tool Registration**: Tools are registered with server name prefix (e.g., `time_get_current_time`)
5. **Tool Execution**: AI model can call MCP tools alongside built-in tools
6. **Shutdown**: MCP servers are terminated when eunice exits

#### Tool Registration and Naming
Each MCP server can expose multiple tools. To avoid naming conflicts and provide clear tool origin, all MCP tools are registered with a server name prefix:

**Naming Convention**: `{server_name}_{tool_name}`

**Examples from configuration:**
- `time` server with `get_current_time` tool → registered as `time_get_current_time`
- `filesystem` server with `read_file` tool → registered as `filesystem_read_file`
- `fetch` server with `fetch` tool → registered as `fetch_fetch`
- `memory` server with `store` tool → registered as `memory_store`
- `email_summarizer` server with `list_configs` tool → registered as `email_summarizer_list_configs`

**Important**: Server names can themselves contain underscores (e.g., `email_summarizer`). The tool routing logic correctly handles this by matching the longest server name prefix.

**Tool Discovery Process:**
1. eunice connects to each configured MCP server
2. Sends `tools/list` request to discover available tools
3. Registers each tool with `{server_name}_{tool_name}` format
4. Presents unified tool list to AI model

**Tool Routing:**
- MCP tools: Routed to appropriate server by matching server name prefix
- Tool calls like `email_summarizer_list_configs` are sent to the `email_summarizer` server
- Server name prefix is stripped before forwarding: `list_configs` sent to server
- Works correctly with server names containing underscores

#### Common MCP Servers and Their Tools
- **Filesystem** (`@modelcontextprotocol/server-filesystem`): `read_file`, `write_file`, `list_directory`, `create_directory`
- **Memory** (`@modelcontextprotocol/server-memory`): `store`, `retrieve`, `search`, `delete`
- **Sequential Thinking** (`@modelcontextprotocol/server-sequential-thinking`): `think`, `reflect`, `summarize`
- **Fetch** (`mcp-server-fetch`): `fetch`, `post`, `get_headers`
- **Time** (`mcp-server-time`): `get_current_time`, `get_timezone`, `format_time`

**Example Tool Registrations:**
```
MCP Tools (with server prefixes):
- filesystem_read_file
- filesystem_write_file
- filesystem_list_directory
- memory_store
- memory_retrieve
- fetch_fetch
- time_get_current_time
- sequential-thinking_sequentialthinking
- email_summarizer_list_configs
- email_summarizer_search_by_config
```

#### Error Handling
- **Invalid configuration files**: Clear error messages with file path and JSON validation details
- **MCP server startup failures**: Detailed error output with command, args, and stderr
- **Tool discovery failures**: Warning messages for servers that don't respond to `tools/list`
- **Tool execution errors**: MCP server errors propagated to AI model with server context
- **Server crashes**: Graceful degradation with error reporting and tool deregistration
- **Tool name conflicts**: Warning when MCP tool names conflict with built-in tools (MCP tools take precedence with prefix)

## Visual Features

### Model Information Display
eunice displays the active model and provider at the start of each session using light yellow framed display:

**Model Information** (Light Yellow):
```
┌─────────────────────────────────────────────────────┐
│ 🤖 Model: llama3.1 (ollama)                        │
└─────────────────────────────────────────────────────┘
```

### MCP Server Information Display
When MCP servers are configured, eunice displays server and tool information at the start of agent output using light yellow framed display:

**MCP Servers & Tools** (Light Yellow):
```
┌────────────────────────────────────────────────┐
│ 🔌 MCP Servers & Tools                         │
├────────────────────────────────────────────────┤
│ 📡 filesystem: 14 tools                       │
│   • filesystem.read_file                      │
│   • filesystem.write_file                     │
│   • filesystem.list_directory                 │
│   • ...and 11 more                            │
│ 📡 memory: 9 tools                            │
│   • memory.create_entities                    │
│   • memory.store                              │
│   • ...and 7 more                             │
│ 📡 time: 2 tools                              │
│   • time.get_current_time                     │
│   • time.convert_time                         │
└────────────────────────────────────────────────┘
```

### Rich Terminal Output
Tool executions are displayed using the Rich library for enhanced visual presentation:

**Tool Invocations** (Bright Blue Panels):
```
╭───────────────────────────────── Tool Call ──────────────────────────────────╮
│ 🔧 filesystem.list_directory({"path":"."})                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**Tool Results** (Green Panels):
```
╭─────────────────────────────────── Result ───────────────────────────────────╮
│ [                                                                            │
│   {"name": "file.txt", "type": "file"},                                     │
│   ...245 characters truncated                                               │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**Model Information** (Yellow Panels):
```
╭───────────────────────────────── Model Info ─────────────────────────────────╮
│ 🤖 Model: llama3.1 (ollama)                                                 │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**MCP Server Information** (Yellow Panels):
```
╭────────────────────────────────── MCP Info ──────────────────────────────────╮
│ 🔌 MCP Servers & Tools                                                      │
│                                                                              │
│ 📡 filesystem: 14 tools                                                     │
│   • filesystem.read_file                                                    │
│   • filesystem.write_file                                                   │
│   • filesystem.list_directory                                               │
│ 📡 memory: 9 tools                                                          │
│   • memory.create_entities                                                  │
│   • memory.store                                                            │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**Model List Display** (Rich Table):
The `--list-models` command displays available models in a structured table format with provider icons, API key status, and organized model listings for better readability.

### Enhanced Help Display
The `--help` command shows:
- Available models grouped by provider with icons (🤖 OpenAI, 💎 Gemini, 🧠 Anthropic, 🦙 Ollama)
- API key status with checkmarks (✅/❌) and last 4 characters
- Locally installed Ollama models
- Smart default model selection information

### Interactive Mode
eunice features an interactive mode for ongoing conversations:

```bash
eunice --interact
```

**Features:**
- **Single startup display**: Model and MCP server information shown once at session start
- **Continuous conversation**: No need to restart for follow-up questions
- **Context preservation**: Maintains conversation history throughout the session
- **Tool persistence**: MCP servers remain active for the entire session
- **Clean interface**: No repeated system information displays between prompts

## Sysadmin Mode

eunice includes a specialized **sysadmin mode** (`--sysadmin`) designed for agentic software engineering tasks. This mode automatically loads a comprehensive set of MCP tools and system instructions to enable the AI to perform complex development workflows including code analysis, file editing, testing, git operations, web research, and more.

### Overview

Sysadmin mode transforms eunice into a full-featured AI coding assistant by:
1. **Auto-loading** a curated set of 7 MCP servers with development-focused tools
2. **Injecting** detailed system instructions that guide the AI to follow best practices
3. **Enabling** agentic workflows like bug fixing, feature implementation, refactoring, and testing

This mode is ideal for developers who want an AI assistant that can autonomously handle complex, multi-step software engineering tasks while adhering to project conventions and best practices.

### Usage

```bash
# Enable sysadmin mode with auto-configured MCP servers
eunice --sysadmin "Fix the bug in main.py where users can't log in"

# Sysadmin mode with interactive conversations
eunice --sysadmin --interact

# Sysadmin mode with specific model
eunice --sysadmin --model="sonnet" "Refactor the authentication system to use JWT tokens"

# Combine with other options
eunice --sysadmin --verbose "Add comprehensive tests for the API endpoints"
```

**Important**: `--sysadmin` cannot be used with `--no-mcp` or `--config` flags, as it provides its own embedded MCP configuration.

### Embedded MCP Configuration

When `--sysadmin` is enabled, eunice automatically loads the following MCP servers (embedded in `eunice.py`):

#### 1. Shell Server (`shell`)
- **Package**: `git+https://github.com/emsi/mcp-server-shell`
- **Tools**: Execute shell commands, run tests, build projects, git operations
- **Use Cases**: Running test suites, executing linters, git workflows, system commands

#### 2. Filesystem Server (`filesystem`)
- **Package**: `@modelcontextprotocol/server-filesystem`
- **Tools**: `read_file`, `write_file`, `list_directory`, `create_directory`, `move_file`, `search_files`
- **Use Cases**: Basic file operations, creating new files, directory navigation

#### 3. Text Editor Server (`text-editor`)
- **Package**: `mcp-text-editor`
- **Tools**: `get_text_file_contents`, `patch_text_file_contents`
- **Features**:
  - **Hash-based conflict detection**: Prevents concurrent modification issues
  - **Line-range reading**: Read specific portions of large files efficiently
  - **Patch operations**: Apply targeted edits to specific line ranges
  - **Bottom-up patching**: Apply multiple patches without line number shifts
- **Use Cases**: **Preferred for editing existing files** - provides safe, efficient modifications with conflict detection

#### 4. Grep Server (`grep`)
- **Package**: `mcp-ripgrep@latest`
- **Tools**: `ripgrep` - Fast code search with regex patterns
- **Features**: Context lines, file filtering, case-insensitive search
- **Use Cases**: Finding code patterns, TODO comments, function definitions across codebase

#### 5. Memory Server (`memory`)
- **Package**: `@modelcontextprotocol/server-memory`
- **Tools**: `create_entities`, `add_observations`, `search_nodes`
- **Storage**: `~/.eunice/` directory
- **Use Cases**: Store user preferences, project facts, remember important context across sessions

#### 6. Web Search Server (`web`)
- **Package**: `@brave/brave-search-mcp-server`
- **Tools**: `web_search` or `brave_web_search`
- **Requirements**: **BRAVE_API_KEY** environment variable ([Get free API key](https://brave.com/search/api/) - 2,000 queries/month free)
- **Use Cases**: Find documentation, research libraries, look up error messages, discover solutions

#### 7. Fetch Server (`fetch`)
- **Package**: `mcp-server-fetch`
- **Tools**: `fetch` - Make HTTP GET/POST requests
- **Use Cases**: Fetch web content, API calls, download resources, read online documentation

### System Instructions

Sysadmin mode loads comprehensive system instructions from `sysadmin_instructions.yml`, which guide the AI to:

#### Core Mandates
- **Conventions**: Rigorously adhere to existing project conventions
- **Libraries/Frameworks**: Never assume availability - verify usage in project first
- **Style & Structure**: Mimic existing code style, formatting, naming, and architectural patterns
- **Idiomatic Changes**: Ensure modifications integrate naturally with local context
- **Comments**: Add sparingly, focus on "why" not "what"
- **Proactiveness**: Fulfill requests thoroughly, including adding tests
- **Confirm Ambiguity**: Don't take significant actions beyond scope without confirmation
- **No Summaries**: Skip summaries after completing tasks unless asked
- **No Reverts**: Don't revert changes unless explicitly requested

#### Primary Workflows

**Software Engineering Tasks:**
1. **Understand & Strategize**: Use filesystem and grep tools to build comprehensive understanding
2. **Plan**: Break complex tasks into subtasks, create coherent plans
3. **Implement**: Use MCP tools to execute plan while following project conventions
4. **Verify (Tests)**: Run project's test suite to validate changes
5. **Verify (Standards)**: Execute linting, type-checking, build commands
6. **Finalize**: Complete task, await next instruction

**New Applications:**
1. Understand requirements and identify core features
2. Propose clear plan with technology stack
3. Obtain user approval
4. Implement features using shell scaffolding tools
5. Verify against requirements, fix bugs
6. Provide startup instructions and request feedback

#### Tool Usage Guidance

**File Operations:**
- Always read files before editing to understand current content
- **For editing existing files**: Use `text-editor_get_text_file_contents` (with hash), then `text-editor_patch_text_file_contents`
- **For creating new files**: Use `filesystem_write_file`
- **For complete rewrites**: Use `filesystem_write_file`
- **For large files**: Use text-editor to read specific line ranges and apply targeted patches
- Text-editor provides conflict detection via hashes - prevents concurrent modification issues

**Code Search:**
- Use `grep_ripgrep` for finding code patterns across multiple files
- Use `filesystem_search_files` for finding files by name/path patterns
- Combine searches in parallel when exploring codebase

**Shell Commands:**
- Explain critical commands before execution
- Use background processes (&) for long-running servers
- Combine related commands with && to save round trips
- Verify changes with appropriate commands (tests, linters, type checkers)

**Git Workflows:**
- Use `git status`, `git diff HEAD`, `git log -n 3` before commits
- Propose draft commit messages focused on "why" not "what"
- Confirm successful commits with `git status`
- Never push changes without explicit user request

### Environment Requirements

To use all sysadmin mode features, ensure you have:

1. **Node.js** (for npx-based MCP servers: filesystem, grep, memory, web)
2. **uv** (for uvx-based MCP servers: shell, text-editor, fetch)
3. **BRAVE_API_KEY** environment variable (optional, for web search)
   - Get free API key at https://brave.com/search/api/
   - 2,000 queries/month on free tier
   - Set via: `export BRAVE_API_KEY="your-key-here"`

### Implementation Details

**Architecture:**
- MCP configuration embedded as Python constant `SYSADMIN_MCP_CONFIG` in `eunice.py`
- System instructions loaded from `sysadmin_instructions.md` (packaged with eunice)
- Instructions read directly as markdown (no parsing/formatting overhead)
- Instructions prepended to first user message only (in both single-shot and interactive modes)

**Instruction Injection:**
```python
if sysadmin and prompt:
    system_instructions = load_sysadmin_instructions()
    prompt = f"""{system_instructions}

---

# USER REQUEST

{prompt}

---

You are now in sysadmin mode. Execute the user request above using your available MCP tools.
"""
```

**Interactive Mode:**
In interactive mode, sysadmin instructions are shown once at session start with a visual indicator:
```
┌─────────────────────────────────────────────┐
│ 🔧 Sysadmin Mode: Active                   │
└─────────────────────────────────────────────┘
```

### Example Workflows

```bash
# Bug fixing with automatic test verification
eunice --sysadmin "Fix the authentication bug where passwords aren't validated"

# Feature implementation with comprehensive approach
eunice --sysadmin "Add a new REST API endpoint for user profile updates with tests"

# Code refactoring following project conventions
eunice --sysadmin "Refactor the database layer to use async/await patterns"

# Research and implementation
eunice --sysadmin "Research best practices for JWT tokens and implement them in our auth system"

# Multi-step development workflow
eunice --sysadmin --interact
> "Analyze the codebase structure"
> "Find all TODO comments and prioritize them"
> "Implement the highest priority TODO with tests"
> "Run the test suite and fix any failures"
```

### Tool Workflow Example

Here's how sysadmin mode typically handles a task like "Fix bug in login.py":

1. **Search**: Use `grep_ripgrep` to find login-related code
2. **Read**: Use `text-editor_get_text_file_contents` to read `login.py` with hash
3. **Understand**: Analyze code to identify bug
4. **Edit**: Use `text-editor_patch_text_file_contents` to apply fix with hash validation
5. **Test**: Use `shell` to run `pytest tests/test_login.py`
6. **Verify**: Use `shell` to run linters/type checkers
7. **Git**: (if requested) Use `shell` for `git add`, `git commit` with meaningful message

### Comparison: Standard vs Sysadmin Mode

| Feature | Standard Mode | Sysadmin Mode |
|---------|---------------|---------------|
| MCP Tools | Manual config required | 7 servers auto-loaded |
| System Instructions | None | Comprehensive coding guidelines |
| File Editing | Via filesystem only | Text-editor with conflict detection |
| Code Search | Manual setup | grep/ripgrep included |
| Shell Access | Manual setup | Shell server included |
| Web Research | Manual setup | Brave search included (with API key) |
| Memory | Manual setup | Persistent memory included |
| Best Practices | Model-dependent | Enforced via instructions |
| Use Case | General queries | Software engineering tasks |

## Technical Implementation

### File Structure
- `eunice.py` - Main executable script with inline dependencies (1,263 lines)
- `sysadmin_instructions.md` - System instructions for --sysadmin mode (packaged with eunice)
- `sysadmin_config.example.json` - Example MCP configuration for sysadmin mode
- `pyproject.toml` - Package configuration for `uv tool install`
- `Makefile` - Comprehensive build system with colored help and commands
- `Dockerfile` - Optimized container image (6 layers, reduced from 9)
- `tests/host.sh` - Optimized test suite (26 tests, 60% faster execution)
- `tests/container.sh` - Docker environment tests
- `tests/container-eunice.sh` - Container-specific eunice tests
- `scripts/reinstall.sh` - Development utility scripts
- `TERMINAL_IMPROVEMENTS.md` - Research on Rich library alternatives (implemented)

### Dependencies
- `openai` - Unified API client for all providers (OpenAI, Gemini via OpenAI-compatible endpoints, Ollama)
- `anthropic` - Anthropic API client for Claude models
- `rich` - Modern terminal UI library for beautiful console output and formatting
- `mcp` - Model Context Protocol client library for MCP server communication (optional)
- Standard library modules: `argparse`, `json`, `os`, `subprocess`, `sys`, `pathlib`, `asyncio`, `urllib.request`

### Process Management
- **MCP Servers**: `subprocess` is used exclusively for starting and managing MCP server processes via `asyncio.create_subprocess_exec()`
- **Ollama Integration**: Uses HTTP API calls to `localhost:11434/api/tags` instead of CLI subprocess calls
- **No External CLI Dependencies**: All external service interactions use proper APIs (HTTP) rather than subprocess calls

### Testing
Comprehensive test coverage including:
- Provider detection and validation (OpenAI, Gemini, Anthropic, Ollama)
- Smart default model selection with priority order
- Tool functionality and MCP server integration
- Interactive mode with proper display handling
- Colored output rendering and silent mode
- Command line argument parsing
- File vs string prompt detection
- API key validation and model routing
- Ollama model validation and availability checking
- Edge cases (e.g., gpt-oss routing, provider priority)

### Error Handling
- Missing API keys: Clear error messages with required environment variables
- Invalid models: Lists available alternatives with installation suggestions
- File errors: Distinguishes between file paths and string prompts
- Ollama integration: Validates model availability via subprocess calls

## Examples

eunice includes practical examples demonstrating various capabilities and use cases:

### Multi-Agent Story Writing (`examples/multi_agent/`)

This sophisticated example demonstrates a multi-agent workflow where eunice orchestrates different "agents" to collaboratively create and refine content:

**Workflow:**
1. **Writer Agent** - Creates initial cyberpunk stories based on prompts
2. **Editor Agent** - Improves story pacing, character development, and narrative flow
3. **Publisher Agent** - Evaluates stories and decides if they meet publication standards
4. **Memory System** - Tracks iterations, feedback, and progress across multiple rounds

**Key Features:**
- Complex multi-step agentic workflows
- File I/O operations via MCP filesystem server
- Memory persistence for tracking state between iterations
- Iterative improvement based on structured feedback
- Automatic retry logic for rejected stories (up to 3 iterations)

**Usage:**
```bash
cd examples/multi_agent && ./run.sh
```

**Files Generated:**
- `story.txt` - Initial story from writer agent
- `story_edited.txt` - Improved version from editor agent
- `story_publisher_result.txt` - Publisher evaluation (TRUE/FALSE/REJECTED)

### Simple Time Operations (`examples/simple_time/`)

Basic time queries demonstrating MCP time server integration:

```bash
cd examples/simple_time && ./run_default_config.sh  # Uses automatic config discovery
cd examples/simple_time && ./run_explicit_config.sh  # Uses explicit config
```

**Demonstrates:**
- Automatic configuration discovery (`eunice.json`)
- Manual configuration specification
- Time/date MCP server integration
- Simple tool calling workflows

### Shell Command Execution (`examples/shell/`)

Execute shell commands through eunice using MCP shell server:

```bash
cd examples/shell && ./test.sh
```

**Shows:**
- Shell command execution via MCP
- Script automation capabilities
- System interaction through eunice
- Integration with external tools and scripts


## Usage Patterns

### File Analysis
```bash
eunice "What type of files are in this directory and what do they do?"
eunice --model="gemini-2.5-pro" "Read the main.py file and explain its purpose"
```

### Development Tasks
```bash
eunice "List all Python files and summarize their contents"
eunice --tool-output-limit=200 "Analyze the project structure"
eunice --config=./dev-tools.json "Check git status and run tests"
```

### MCP-Enhanced Tasks
```bash
# With multiple MCP servers (filesystem, memory, time, fetch)
eunice --config=./config.example.json "What time is it and what files are in this directory?"

# With memory server for persistent context
eunice --config=./config.example.json "Store this project analysis in memory for later reference"

# With fetch server for web requests
eunice --config=./config.example.json "Fetch the latest news from the GitHub API and summarize it"

# Combining multiple MCP tools
eunice --config=./config.example.json "Get the current time, list files, and store a summary in memory"
```

**Example Tool Calls Generated:**
```json
[
  {"type": "function", "function": {"name": "time.get_current_time"}},
  {"type": "function", "function": {"name": "filesystem.list_directory", "arguments": {"path": "."}}},
  {"type": "function", "function": {"name": "memory.store", "arguments": {"key": "project_summary", "value": "..."}}}
]
```

### Model Management
```bash
eunice --list-models                    # See all available models
eunice --help                          # Check API key status
```

### Docker Usage
```bash
# List available models (connects to host Ollama API)
docker run --rm --network host xebxeb/eunice eunice --list-models

# Use Ollama models (connects to host Ollama web API at localhost:11434)
docker run --rm --network host xebxeb/eunice eunice --model="gpt-oss" "What is best in life?"

# Use cloud models with API keys
docker run --rm -e OPENAI_API_KEY="$OPENAI_API_KEY" xebxeb/eunice eunice --model="gpt-4" "Hello world"
docker run --rm -e GEMINI_API_KEY="$GEMINI_API_KEY" xebxeb/eunice eunice --model="gemini-2.5-flash" "Hello world"

# Docker with MCP servers (mount config and working directory)
docker run --rm -v "$(pwd)":/workspace -w /workspace xebxeb/eunice eunice --config=./eunice.json "List files and get current time"
```

#### Bash Alias for Convenient Docker Usage

The `xebxeb/eunice` Docker image includes a default MCP configuration at `/root/eunice.json` (based on `config.example.json`). You can add this bash alias to `~/.bash_aliases` for convenient dockerized usage:

```bash
# Eunice Docker
eud() {
    local base_cmd="docker run -e OPENAI_API_KEY -e GEMINI_API_KEY -e ANTHROPIC_API_KEY --network host --rm -it"
    if [[ "$1" == "-p" ]]; then
        shift
        $base_cmd -v "$(pwd)":/work -w /work -e OLLAMA_HOST=http://localhost:11434 xebxeb/eunice eunice --config=/root/eunice.json "$@"
    else
        $base_cmd -e OLLAMA_HOST=http://localhost:11434 xebxeb/eunice eunice --config=/root/eunice.json "$@"
    fi
}
```

**Usage:**
```bash
# Run without mounting current directory
eud "What time is it?"

# Run with current directory mounted as /work (-p flag)
eud -p "List files in the current directory"
```

**Features:**
- Automatically passes through API keys for OpenAI, Gemini, and Anthropic
- Uses the default MCP configuration at `/root/eunice.json`
- `-p` flag mounts current directory as `/work` for file operations
- Connects to host Ollama instance via `localhost:11434`

## Architecture Decisions

### Why OpenAI API Format?
Using the OpenAI API specification as the common interface allows:
- Unified client code across all providers
- Consistent tool calling interface
- Easy provider switching
- Compatibility with Ollama's OpenAI-compatible endpoint

### Tool Design Philosophy
The framework supports both built-in and extensible tool approaches:
- **Built-in tools**: Minimal, hardcoded file operations for safety and reliability
- **MCP tools**: Extensible via configuration, enabling rich integrations
- **Unified interface**: All tools appear identical to AI models
- **Structured responses**: Consistent JSON/text formatting across tool types
- **Local + network**: Built-in tools are local-only; MCP tools can access network resources

### Provider Priority
The detection logic prioritizes actual model availability over name patterns:
1. Exact provider match (gemini*)
2. Local availability check (Ollama)
3. Pattern matching (OpenAI)
4. Error with suggestions

This prevents issues like `gpt-oss` being misrouted to OpenAI when it's an Ollama model.

## Development Workflow

### Build System
eunice includes a comprehensive Makefile for development tasks:

```bash
make help           # Show all available commands with descriptions
make test-host      # Fast local tests (60% faster with optimizations)
make test-docker    # Docker environment tests
make test           # Run all available tests
make build-and-test # Build Docker image and run tests
make install        # Install eunice locally
make reinstall      # Reinstall for development
make publish        # Push to Docker registry
make clean          # Clean up temporary files
```

### Running Tests
```bash
# Optimized local testing (recommended for development)
make test-host      # 26 tests with --no-mcp optimizations (60% faster)

# Docker testing for clean environment validation
make test-docker    # Full Docker environment tests
```

#### Test Coverage
- **26 comprehensive tests** covering all functionality
- Provider detection (OpenAI, Gemini, Anthropic, Ollama) with smart defaults
- Model validation and routing with priority order
- MCP server integration and tool functionality
- Interactive mode with proper startup display handling
- Colored output and visual display features
- Silent mode operation and verbose debugging
- Error handling and edge cases
- Command line argument parsing
- Long prompt handling and template characters
- **Performance optimized**: 60% faster execution with selective --no-mcp flags

#### Docker Testing Environment
The Docker test setup provides:
- **Clean environment testing** using Alpine Linux
- **Optimized layers** (reduced from 9 to 6 layers)
- **Host Ollama connectivity** via port binding to localhost:11434
- **API key pass-through** for OpenAI/Gemini/Anthropic testing
- **Isolated dependency validation** ensuring clean installs work
- **Comprehensive test execution** with full suite coverage

Docker test configuration:
- Uses `host.docker.internal:host-gateway` for host connectivity
- Sets `OLLAMA_HOST="http://host.docker.internal:11434"` for model access
- Copies only essential files: `eunice.py`, `pyproject.toml`, `tests/`, `README.md`
- Validates `uv tool install .` works in clean environment

### Local Development
```bash
# Direct execution for testing
uv run eunice.py "test prompt"  # Uses smart default model
uv run eunice.py --model="llama3.1" "test prompt"
uv run eunice.py --silent "quiet operation"
uv run eunice.py --interact  # Interactive mode testing

# Development workflow
make dev            # Development mode
make reinstall      # Quick reinstall for testing changes
```

### Installation Testing
```bash
uv tool install .
eunice --help
eunice --list-models  # Verify model detection
uv tool uninstall eunice
```

## Future Considerations

The framework is designed to be extensible while maintaining simplicity:
- **Rich Library Integration**: Research completed on terminal display improvements using Rich library (see `TERMINAL_IMPROVEMENTS.md`)
- **Additional Provider Support**: Framework designed to easily add new AI providers through unified client approach
- **Enhanced MCP Integration**: Streamlined MCP server management and tool discovery
- **Performance Optimizations**: Continued test suite improvements and execution speed enhancements
- **Interactive Enhancements**: Potential for more advanced conversation management and context handling

**Recent Improvements:**
- ✅ Smart default model selection with priority ordering
- ✅ Interactive mode with clean startup display
- ✅ Test suite optimization (60% faster execution)
- ✅ Docker layer optimization (9 → 6 layers)
- ✅ Comprehensive build system with Makefile
- ✅ Anthropic API integration with model aliases
- ✅ Enhanced error handling and model routing
- ✅ **Rich library integration** - Professional terminal UI with panels, tables, and enhanced formatting
- ✅ **Sysadmin mode** - Full-featured AI coding assistant with 7 auto-loaded MCP servers, comprehensive system instructions, and specialized tools for software engineering tasks (1,263 lines total)

The core philosophy remains: provide a simple, reliable interface for agentic AI interactions while continuously improving performance and user experience.