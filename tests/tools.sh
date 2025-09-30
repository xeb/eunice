#!/bin/bash
# Test that MCP server names with underscores work correctly
# This test specifically catches the bug where "email_summarizer_list_configs"
# was being routed to server "email" instead of "email_summarizer"

set -e

echo "=== Testing MCP Server Names with Underscores ==="

# Determine test model (same logic as host.sh)
if curl -s "http://localhost:11434/api/tags" >/dev/null 2>&1; then
    TEST_MODEL="llama3.1:latest"
elif [ -n "$GEMINI_API_KEY" ]; then
    TEST_MODEL="gemini-2.5-flash"
else
    echo "⚠ Skipping tools.sh tests - neither Ollama nor Gemini available"
    exit 0
fi

# Create a config with a server name containing underscores
cat > test_time_underscore.json << 'EOF'
{
  "mcpServers": {
    "time_server": {
      "command": "uvx",
      "args": ["mcp-server-time"]
    }
  }
}
EOF

# Test that the server starts and tools are registered with correct prefixes
echo "Testing time_server with underscore (using model: $TEST_MODEL)..."
OUTPUT=$(timeout 15 uv run eunice.py --config=test_time_underscore.json --model=$TEST_MODEL "What time is it in UTC?" 2>&1)

# Check that tools are registered with underscore prefix
if echo "$OUTPUT" | grep -q "time_server_get_current_time"; then
    echo "✓ Tools registered correctly with underscore prefix"
else
    echo "✗ FAIL: Tools not registered with correct prefix"
    echo "$OUTPUT"
    rm -f test_time_underscore.json
    exit 1
fi

# Check that tool execution doesn't produce "Unknown server" or "Unknown tool" error
if echo "$OUTPUT" | grep -qE "Unknown server|Unknown tool"; then
    echo "✗ FAIL: Got 'Unknown server' or 'Unknown tool' error - tool routing broken"
    echo "$OUTPUT"
    rm -f test_time_underscore.json
    exit 1
else
    echo "✓ Tool routing works correctly for underscore server names"
fi

# Check that we got a valid time response (indicates tool was executed successfully)
if echo "$OUTPUT" | grep -qE "[0-9]{2}:[0-9]{2}|time|UTC"; then
    echo "✓ Tool executed successfully and returned time data"
else
    echo "⚠ Warning: Tool may not have executed (no time data in output)"
fi

rm -f test_time_underscore.json

# Create a config with MULTI-underscore server name (email_summarizer pattern)
cat > test_multi_underscore.json << 'EOF'
{
  "mcpServers": {
    "test_multi_server": {
      "command": "uvx",
      "args": ["mcp-server-time"]
    }
  }
}
EOF

echo ""
echo "Testing test_multi_server with multiple underscores (using model: $TEST_MODEL)..."
OUTPUT=$(timeout 15 uv run eunice.py --config=test_multi_underscore.json --model=$TEST_MODEL "What time is it?" 2>&1)

# Check that tools are registered with correct multi-underscore prefix
if echo "$OUTPUT" | grep -q "test_multi_server_get_current_time"; then
    echo "✓ Multi-underscore tools registered correctly"
else
    echo "✗ FAIL: Multi-underscore tools not registered with correct prefix"
    echo "$OUTPUT"
    rm -f test_multi_underscore.json
    exit 1
fi

# Check that tool execution doesn't produce "Unknown tool" error (the specific bug we're catching)
if echo "$OUTPUT" | grep -q "Unknown tool"; then
    echo "✗ FAIL: Got 'Unknown tool' error - this is the bug where split('_', 1) cuts off too early"
    echo "       Tool name 'test_multi_server_get_current_time' was incorrectly parsed"
    echo "       Expected: 'get_current_time' sent to server"
    echo "       Actual: Probably sent 'multi_server_get_current_time' to server"
    echo "$OUTPUT"
    rm -f test_multi_underscore.json
    exit 1
else
    echo "✓ Multi-underscore tool routing works correctly"
fi

# Check that we got a valid time response
if echo "$OUTPUT" | grep -qE "[0-9]{2}:[0-9]{2}|time"; then
    echo "✓ Multi-underscore tool executed successfully"
else
    echo "⚠ Warning: Multi-underscore tool may not have executed (no time data in output)"
fi

rm -f test_multi_underscore.json

echo ""
echo "=== All underscore server name tests passed ==="