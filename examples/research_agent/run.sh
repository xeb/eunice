#!/bin/bash
# run.sh - Run the research agent example

set -e

# Ensure eunice is available
if ! command -v eunice &> /dev/null; then
    EUNICE_PATH="../../target/release/eunice"
    if [ ! -f "$EUNICE_PATH" ]; then
        echo "eunice binary not found. Please build with 'cargo build --release'"
        exit 1
    fi
else
    EUNICE_PATH="eunice"
fi

# Check for mcpz (filesystem MCP server)
if ! command -v mcpz &> /dev/null; then
    echo "Error: mcpz not found. Install with: cargo install mcpz"
    exit 1
fi

# Check for GEMINI_API_KEY
if [ -z "$GEMINI_API_KEY" ]; then
    echo "Error: GEMINI_API_KEY environment variable not set"
    echo "The search_query tool requires a Gemini API key."
    exit 1
fi

# Create output directories
mkdir -p research_notes reports

echo "=== Research Agent ==="
echo "Using eunice multi-agent orchestration with search_query tool"
echo ""
echo "Agents: root (lead), researcher, report_writer"
echo "Output: research_notes/, reports/"
echo ""
echo "Example queries:"
echo "  - Research quantum computing developments"
echo "  - What are current trends in renewable energy?"
echo "  - Research the Detroit Lions 2025 season"
echo ""

# Run eunice in DMN mode with interactive
"$EUNICE_PATH" --dmn -i
