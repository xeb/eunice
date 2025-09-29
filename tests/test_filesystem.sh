#!/bin/bash
# Test filesystem MCP server with eunice

set -e

echo "===== Testing Filesystem MCP Server ====="
echo

# Check if ANTHROPIC_API_KEY is set
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "ERROR: ANTHROPIC_API_KEY not set. Please set it first."
    echo "export ANTHROPIC_API_KEY='your-key-here'"
    exit 1
fi

# Check if config.example.json exists
if [ ! -f "config.example.json" ]; then
    echo "ERROR: config.example.json not found"
    exit 1
fi

echo "Test 1: How many files in this directory?"
echo "Command: uv run eunice.py --config=config.example.json --model=sonnet-4.5 'how many files in this directory?'"
echo
timeout 30 uv run eunice.py --config=config.example.json --model=sonnet-4.5 "how many files in this directory?" 2>&1
echo
echo "========================"
echo

echo "Test 2: Read README.md"
echo "Command: uv run eunice.py --config=config.example.json --model=sonnet-4.5 'read the README.md file and summarize it in one sentence'"
echo
timeout 30 uv run eunice.py --config=config.example.json --model=sonnet-4.5 "read the README.md file and summarize it in one sentence" 2>&1
echo
echo "========================"
echo

echo "Test 3: List files (more explicit)"
echo "Command: uv run eunice.py --config=config.example.json --model=sonnet-4.5 'use the filesystem tools to list all files in the current directory'"
echo
timeout 30 uv run eunice.py --config=config.example.json --model=sonnet-4.5 "use the filesystem tools to list all files in the current directory" 2>&1
echo
echo "========================"
echo

echo "All tests complete!"