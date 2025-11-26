# Shell Inspection Example

This example demonstrates using Eunice with a single MCP server to perform comprehensive system reconnaissance. The agent executes shell commands to gather detailed information about the host system and compiles the findings into a structured markdown report.

## What It Does

The agent acts as a system reconnaissance tool that:

1. Executes a wide range of shell commands to gather system information
2. Collects data about hardware, network, users, services, and security configuration
3. Compiles all findings into `workspace/inspection.md`

Categories of information collected:
- System identity (hostname, machine-id)
- Operating system and kernel details
- Hardware (CPU, memory, storage)
- Network configuration and external IP
- Open ports and running services
- User accounts and authentication
- Installed software and development tools
- Security posture (firewall, SELinux/AppArmor)
- Potential vulnerabilities

## Prerequisites

1. Install Eunice:
   ```bash
   cargo install eunice
   ```

2. Install the mcpz shell server:
   ```bash
   cargo install mcpz
   ```

3. Set up an API key for your preferred AI provider (e.g., `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, or `GEMINI_API_KEY`)

## Usage

From this directory, simply run:

```bash
eunice
```

Eunice will automatically:
- Load the `eunice.json` configuration (MCP shell server)
- Read the `instruction.md` file as the system prompt
- Begin executing shell commands and gathering system information
- Create `workspace/inspection.md` with the compiled report

## Configuration

### eunice.json

```json
{
  "mcpServers": {
    "shell": {
      "command": "mcpz",
      "args": ["server", "shell"]
    }
  }
}
```

This configures a single MCP server providing shell execution capabilities via [mcpz](https://github.com/xelamonster/mcpz).

### instruction.md

Contains detailed instructions for the agent specifying:
- Which shell commands to execute
- How to handle command failures
- The expected output format for the report

## Output

After running, you'll find a comprehensive system report at `workspace/inspection.md` containing:
- Executive summary
- Detailed system information organized by category
- Security observations and potential vulnerabilities
- Raw command outputs for reference

## Security Note

This example performs extensive system reconnaissance. Run only on systems you own or have explicit authorization to inspect. The commands are read-only and non-destructive, but they do reveal sensitive system information.
