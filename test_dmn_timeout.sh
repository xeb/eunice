#!/bin/bash
# Test script to reproduce MCP server timeout issues in DMN mode
# This creates a scenario with multiple sequential tool calls and
# commands that might generate stderr output

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="/tmp/eunice_dmn_timeout_test"

echo "=== DMN Mode Timeout Reproduction Test ==="
echo ""

# Clean up and create work directory
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"
echo "Work directory: $WORK_DIR"
echo ""

# Create a test prompt that does many sequential operations
# This is designed to stress-test the MCP server communication
PROMPT=$(cat << 'EOF'
Execute these commands one at a time in order. After each command, immediately execute the next one:

1. echo "Test 1" > /tmp/eunice_dmn_timeout_test/test1.txt
2. ls -la /tmp/eunice_dmn_timeout_test/
3. echo "Test 2" > /tmp/eunice_dmn_timeout_test/test2.txt
4. cat /tmp/eunice_dmn_timeout_test/test1.txt /tmp/eunice_dmn_timeout_test/test2.txt
5. echo "Test 3: $(date)" > /tmp/eunice_dmn_timeout_test/test3.txt
6. find /tmp/eunice_dmn_timeout_test -type f -name "*.txt"
7. wc -l /tmp/eunice_dmn_timeout_test/*.txt
8. echo "Test complete"

Do NOT stop or ask for confirmation. Execute all 8 commands sequentially.
After completing all commands, say "DMN TIMEOUT TEST COMPLETE"
EOF
)

echo "Running eunice in DMN mode..."
echo "This will make 8+ sequential tool calls to test for timeouts."
echo ""
echo "---"

# Set a timeout for the whole test (2 minutes should be plenty)
timeout 120 eunice --dmn "$PROMPT" 2>&1 || {
    EXIT_CODE=$?
    echo ""
    echo "---"
    if [ $EXIT_CODE -eq 124 ]; then
        echo "*** TEST FAILED: eunice timed out after 120 seconds ***"
        echo "This indicates an MCP server communication issue."
    else
        echo "*** TEST FAILED: eunice exited with code $EXIT_CODE ***"
    fi
    exit 1
}

echo ""
echo "---"
echo ""

# Verify the results
echo "=== Verification ==="
if [ -f "$WORK_DIR/test3.txt" ]; then
    echo "SUCCESS: All test files were created"
    echo ""
    echo "Files created:"
    ls -la "$WORK_DIR"
    echo ""
    echo "*** DMN TIMEOUT TEST PASSED ***"
else
    echo "PARTIAL: Not all files were created"
    echo "Files in work dir:"
    ls -la "$WORK_DIR" 2>/dev/null || echo "(empty or missing)"
    exit 1
fi

echo ""
echo "Test complete. Work files in: $WORK_DIR"
