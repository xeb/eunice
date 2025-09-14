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
Currently supports two hardcoded tools:
- **`list_files(path: str)`** - Lists files and directories at the specified path
- **`read_file(path: str)`** - Reads and returns the contents of a file

Both tools return structured data that the AI models can process and act upon.

### Provider Support
- **OpenAI**: gpt-3.5-turbo, gpt-4, gpt-4o, gpt-4-turbo, gpt-5, chatgpt-4o-latest
- **Google Gemini**: gemini-2.5-flash, gemini-2.5-pro, gemini-1.5-flash, gemini-1.5-pro
- **Ollama**: Any locally installed model (validated via `ollama list`)

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

## Configuration

### Environment Variables
Required API keys must be set as environment variables:
- `OPENAI_API_KEY` - Required for OpenAI models
- `GEMINI_API_KEY` - Required for Gemini models
- Ollama models run locally and don't require API keys

### Provider Detection Logic
1. **Gemini**: Models starting with "gemini" → Gemini API
2. **Ollama**: Check local availability via `ollama list` (highest priority for installed models)
3. **OpenAI**: Models matching patterns (gpt*, chatgpt*, etc.) → OpenAI API
4. **Fallback**: Unknown models default to Ollama with validation

This ensures models like `gpt-oss` (an Ollama model) are correctly routed to Ollama instead of OpenAI.

## Command Line Interface

### Basic Usage
```bash
eunice "How many files are in the current directory?"
eunice --model="gpt-4" "analyze this codebase"
eunice --model="gemini-2.5-pro" --prompt=./analysis_request.txt
```

### Options
- `--model=MODEL` - Specify AI model (default: gpt-3.5-turbo)
- `--prompt=PROMPT` - Prompt as named argument (can be file path or string)
- `--tool-output-limit=N` - Limit tool output display (default: 50, 0 = no limit)
- `--list-models` - Show all available models grouped by provider
- `--help` - Enhanced help with model availability and API key status

### Prompt Handling
- Prompts can be provided as positional arguments or via `--prompt`
- Automatic file detection: if prompt looks like a file path and exists, content is read
- Supports both direct strings and file-based prompts

## Visual Features

### Colored Tool Output
Tool executions are displayed with colored, framed output:

**Tool Invocations** (Light Blue):
```
┌────────────────────────────────────────────────┐
│ 🔧 list_files({"path":"."})                     │
└────────────────────────────────────────────────┘
```

**Tool Results** (Green):
```
┌────────────────────────────────────────────────┐
│ Result:                                         │
├────────────────────────────────────────────────┤
│ [                                             │
│   {"name": "file.txt", "type": "file"},       │
│   ...245 characters truncated                 │
└────────────────────────────────────────────────┘
```

### Enhanced Help Display
The `--help` command shows:
- Available models grouped by provider with icons (🤖 OpenAI, 💎 Gemini, 🦙 Ollama)
- API key status with checkmarks (✅/❌) and last 4 characters
- Locally installed Ollama models

## Technical Implementation

### File Structure
- `eunice.py` - Main executable script with inline dependencies
- `pyproject.toml` - Package configuration for `uv tool install`
- `test.sh` - Comprehensive test suite (32 tests)

### Dependencies
- `openai` - Unified API client for all providers (OpenAI, Gemini via OpenAI-compatible endpoints, Ollama)
- Standard library modules: `argparse`, `json`, `os`, `subprocess`, `sys`, `pathlib`

### Testing
Comprehensive test coverage including:
- Provider detection and validation
- Tool functionality
- Colored output rendering
- Command line argument parsing
- File vs string prompt detection
- API key validation
- Ollama model validation
- Edge cases (e.g., gpt-oss routing)

### Error Handling
- Missing API keys: Clear error messages with required environment variables
- Invalid models: Lists available alternatives with installation suggestions
- File errors: Distinguishes between file paths and string prompts
- Ollama integration: Validates model availability via subprocess calls

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
```

### Model Management
```bash
eunice --list-models                    # See all available models
eunice --help                          # Check API key status
```

## Architecture Decisions

### Why OpenAI API Format?
Using the OpenAI API specification as the common interface allows:
- Unified client code across all providers
- Consistent tool calling interface
- Easy provider switching
- Compatibility with Ollama's OpenAI-compatible endpoint

### Tool Design Philosophy
Tools are intentionally minimal and hardcoded to maintain simplicity:
- Only file operations that are safe and commonly needed
- Structured JSON responses for consistent parsing
- Local execution only (no network calls from tools)

### Provider Priority
The detection logic prioritizes actual model availability over name patterns:
1. Exact provider match (gemini*)
2. Local availability check (Ollama)
3. Pattern matching (OpenAI)
4. Error with suggestions

This prevents issues like `gpt-oss` being misrouted to OpenAI when it's an Ollama model.

## Development Workflow

### Running Tests
```bash
./test.sh  # Full test suite (32 tests)
```

### Local Development
```bash
uv run eunice.py --model="llama3.1" "test prompt"
```

### Installation Testing
```bash
uv tool install .
eunice --help
uv tool uninstall eunice
```

## Future Considerations

The framework is designed to be extensible while maintaining simplicity:
- Additional tools can be added to the hardcoded tool set
- Provider support can be expanded through the unified OpenAI client
- Output formatting can be enhanced while preserving the colored display system
- Model validation can be extended to other local providers

The core philosophy remains: provide a simple, reliable interface for agentic AI interactions with local file system access.