#!/bin/bash

echo "=== Testing eunice with MCP servers in container ==="

# Test eunice with MCP configuration
echo "Running: eunice --model=\"gpt-oss:latest\" \"List out how many tools you have, just the number of tools\""
OUTPUT=$(eunice --model="gpt-oss:latest" "List out how many tools you have, just the number of tools" 2>&1)

echo "Output: $OUTPUT"

# Extract number from output - look for patterns like "26", "26 tools", etc.
# Get the last line which should contain just the number
TOOL_COUNT=$(echo "$OUTPUT" | tail -1 | grep -oE '[0-9]+' | head -1)

echo "Detected tool count: $TOOL_COUNT"

# Assert that we have at least 3 tools (flexible for MCP server availability)
# In Docker environments, some MCP servers may fail to start due to network/dependency issues
if [ "$TOOL_COUNT" -ge 3 ]; then
    echo "✅ SUCCESS: Found $TOOL_COUNT tools from MCP servers (basic functionality working)"
    exit 0
else
    echo "❌ FAILURE: Expected at least 3 tools, but got $TOOL_COUNT"
    echo "Full output:"
    echo "$OUTPUT"
    exit 1
fi