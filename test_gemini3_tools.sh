#!/bin/bash
# Test script to verify gemini-3-pro-preview tool calling with thought signatures
# This creates a looping task that requires multiple tool calls to prove
# thought signatures are being passed correctly across turns.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/test_mcp_config.json"
WORK_DIR="/tmp/eunice_gemini3_test"

echo "=== Gemini 3 Pro Preview Tool Calling Test ==="
echo "Testing recursive tool calls with thought signature preservation"
echo ""

# Create minimal MCP config with just shell
cat > "$CONFIG_FILE" << 'EOF'
{
  "mcpServers": {
    "shell": {
      "command": "uvx",
      "args": ["git+https://github.com/emsi/mcp-server-shell"]
    }
  }
}
EOF

echo "Created MCP config: $CONFIG_FILE"

# Clean up and create work directory
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"
echo "Work directory: $WORK_DIR"
echo ""

# Create the test prompt - a multi-step task requiring several tool calls
PROMPT=$(cat << 'EOF'
You are testing tool calling. Execute these steps IN ORDER, one at a time:

1. Run: echo "Step 1: $(date +%H:%M:%S)" > /tmp/eunice_gemini3_test/step1.txt
2. Run: echo "Step 2: $(date +%H:%M:%S)" > /tmp/eunice_gemini3_test/step2.txt
3. Run: echo "Step 3: $(date +%H:%M:%S)" > /tmp/eunice_gemini3_test/step3.txt
4. Run: cat /tmp/eunice_gemini3_test/step1.txt /tmp/eunice_gemini3_test/step2.txt /tmp/eunice_gemini3_test/step3.txt > /tmp/eunice_gemini3_test/combined.txt
5. Run: wc -l /tmp/eunice_gemini3_test/combined.txt && cat /tmp/eunice_gemini3_test/combined.txt

After completing all 5 steps, say "TEST COMPLETE - All 5 tool calls succeeded!"

Do NOT explain, just execute each command.
EOF
)

echo "Running eunice with gemini-3-pro-preview..."
echo "This will make 5+ sequential tool calls to test thought signature handling."
echo ""
echo "---"

# Run eunice with the test
eunice \
  --model gemini-3-pro-preview \
  --config "$CONFIG_FILE" \
  "$PROMPT"

echo ""
echo "---"
echo ""

# Verify the results
echo "=== Verification ==="
if [ -f "$WORK_DIR/combined.txt" ]; then
    echo "SUCCESS: combined.txt was created"
    echo "Contents:"
    cat "$WORK_DIR/combined.txt"
    LINES=$(wc -l < "$WORK_DIR/combined.txt")
    echo ""
    echo "Line count: $LINES"
    if [ "$LINES" -eq 3 ]; then
        echo ""
        echo "*** TEST PASSED: All tool calls completed successfully! ***"
        echo "Thought signatures were correctly passed across $LINES+ turns."
    else
        echo "WARNING: Expected 3 lines, got $LINES"
    fi
else
    echo "FAILED: combined.txt was not created"
    echo "Files in work dir:"
    ls -la "$WORK_DIR" 2>/dev/null || echo "(empty)"
    exit 1
fi

# Cleanup
rm -f "$CONFIG_FILE"
echo ""
echo "Test complete. Work files in: $WORK_DIR"
