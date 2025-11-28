#!/bin/bash
# Test script to verify multi-agent interactive mode works correctly
#
# This test demonstrates that:
# 1. Running 'eunice' in a directory with agents should start the root agent
# 2. Interactive mode should show sub-agent invocations (like single-shot mode)
#
# Expected behavior:
# - Should see "🤖 Multi-Agent Mode: starting as 'root'"
# - Should see "🤖 Agent 'head_chef' starting task:" when order is placed
# - Should see sub-agent steps with indentation
#
# Current bug: Interactive mode doesn't use the orchestrator, so no multi-agent output

set -e

cd "$(dirname "$0")"

echo "=== Test: Multi-Agent Interactive Mode ==="
echo ""
echo "This test verifies that interactive mode with agents configured:"
echo "  1. Starts the root agent automatically"
echo "  2. Shows sub-agent invocations like single-shot mode"
echo ""

# Check if expect is available
if ! command -v expect &> /dev/null; then
    echo "ERROR: 'expect' is required for this test but not installed"
    echo "Install with: sudo apt install expect"
    exit 1
fi

# Clean up any previous test artifacts
rm -f orders.txt kitchen_log.txt pantry.txt

echo "Running interactive test with expect..."
echo ""

# Use expect to interact with eunice
expect << 'EOF'
set timeout 120

spawn eunice

# Wait for the prompt
expect {
    "> " {
        # Good - got the prompt
    }
    timeout {
        puts "TIMEOUT waiting for prompt"
        exit 1
    }
}

# Send the order
send "order a cheeseburger\r"

# Wait for response - should see multi-agent activity
expect {
    -re "Agent.*head_chef.*starting" {
        puts "\n✅ PASS: Saw head_chef agent invocation"
    }
    -re "invoke_head_chef" {
        puts "\n✅ PASS: Saw invoke_head_chef tool call"
    }
    "> " {
        puts "\n❌ FAIL: Got prompt without seeing multi-agent activity"
        puts "Interactive mode is not using the orchestrator!"
    }
    timeout {
        puts "\nTIMEOUT waiting for response"
        exit 1
    }
}

# Exit cleanly
send "exit\r"
expect eof
EOF

echo ""
echo "=== Test Complete ==="

# Check if files were created (indicates the agents worked)
if [ -f "orders.txt" ]; then
    echo "✅ orders.txt was created"
    cat orders.txt
else
    echo "❌ orders.txt was NOT created"
fi
