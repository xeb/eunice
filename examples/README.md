# eunice Examples

This directory contains practical examples demonstrating various eunice capabilities and use cases. Each example is self-contained with its own configuration, scripts, and documentation.

## 📚 Available Examples

### 🤖 [Multi-Agent Story Writing](multi_agent/)
**Complexity**: Advanced | **MCP Servers**: filesystem, memory

A sophisticated example showcasing multi-agent workflows where eunice orchestrates different "agents" to collaboratively create and refine cyberpunk stories.

**Features Demonstrated:**
- Complex multi-step agentic workflows
- File I/O operations via MCP filesystem server
- Memory persistence for tracking state between iterations
- Iterative improvement based on structured feedback
- Automatic retry logic for quality assurance

**Usage:** `cd multi_agent && ./run.sh`

---

### ⏰ [Simple Time Operations](simple_time/)
**Complexity**: Beginner | **MCP Servers**: time

Basic time queries demonstrating MCP time server integration and configuration discovery.

**Features Demonstrated:**
- Automatic configuration discovery (`eunice.json`)
- Manual configuration specification
- Time/date MCP server integration
- Simple tool calling workflows

**Usage:**
- `cd simple_time && ./run_default_config.sh` (automatic config)
- `cd simple_time && ./run_explicit_config.sh` (explicit config)

---

### 🖥️ [Shell Command Execution](shell/)
**Complexity**: Intermediate | **MCP Servers**: shell

Execute shell commands through eunice using the MCP shell server for system automation.

**Features Demonstrated:**
- Shell command execution via MCP
- Script automation capabilities
- System interaction through eunice
- Security considerations for shell access

**Usage:** `cd shell && ./test.sh`

⚠️ **Security Note**: The shell MCP server provides powerful system access. Use with caution.

---

## 🚀 Getting Started

1. **Start with Simple Time** - Perfect introduction to eunice basics
2. **Try Shell Commands** - Learn system integration capabilities
3. **Explore Multi-Agent** - Advanced workflows and complex orchestration

## 📋 Prerequisites

- **eunice installed**: `uv tool install git+https://github.com/xeb/eunice`
- **API keys set**: Export your preferred AI service API key
- **Node.js/npm**: Required for MCP servers (most examples use `npx`)
- **uv/uvx**: Required for Python-based MCP servers

## 🔧 Configuration Files

Each example includes its own MCP configuration:
- `eunice.json` - Standard configuration file (auto-discovered)
- `config.json` - Custom configuration file names
- `eunice_minimal.json` - Reduced server sets for stability

## 📖 Learn More

- **[Main Documentation](../README.md)** - Complete eunice guide
- **[Technical Details](../CLAUDE.md)** - In-depth implementation details
- **[config.example.json](../config.example.json)** - Comprehensive MCP configuration reference

## 💡 Creating Your Own Examples

1. Create a new directory under `examples/`
2. Add your MCP configuration file (`eunice.json`)
3. Write your prompts and scripts
4. Create a `README.md` documenting the example
5. Test thoroughly and ensure it works reliably

---

*Each example is designed to be educational and practical - perfect starting points for your own eunice projects!*