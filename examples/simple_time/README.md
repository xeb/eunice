# Simple Time Operations Example

This example demonstrates basic time queries using the MCP time server.

## Usage

### Default Configuration (Automatic Discovery)
```bash
./run_default_config.sh
```
Uses automatic config discovery - eunice finds and loads `eunice.json` automatically.

### Explicit Configuration
```bash
./run_explicit_config.sh
```
Explicitly specifies the config file path.

## Configuration

Uses `eunice.json` with:
- **time** - MCP time server for date/time operations

## What It Demonstrates

- Automatic configuration discovery (`eunice.json`)
- Manual configuration specification
- Time/date MCP server integration
- Simple tool calling workflows
- Basic eunice setup and usage patterns

This is a perfect starting point for understanding eunice basics before moving to more complex examples.