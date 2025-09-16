# Shell Command Execution Example

This example demonstrates shell command execution through eunice using the MCP shell server.

## Usage

```bash
./test.sh
```

This will:
1. Load the shell MCP server configuration
2. Ask eunice to run `sample_script.sh`
3. Report whether the script executed successfully

## Configuration

Uses `config.json` with:
- **shell** - MCP shell server for executing system commands

## Files

- `test.sh` - Main test script that runs the example
- `sample_script.sh` - Simple script that outputs "I guess I ran"
- `config.json` - MCP configuration for shell server

## What It Demonstrates

- Shell command execution via MCP
- Script automation capabilities
- System interaction through eunice
- Integration with external tools and scripts
- Security considerations for shell access

## Security Note

The shell MCP server provides powerful system access. Use with caution and ensure proper security measures in production environments.