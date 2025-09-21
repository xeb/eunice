#!/bin/bash

echo "=== Running Container Test Suite ==="

# First run the host tests (original test suite)
echo ""
echo "1. Running host test suite (test-host.sh)..."
./test-host.sh
HOST_EXIT_CODE=$?

if [ $HOST_EXIT_CODE -eq 0 ]; then
    echo "✅ Host tests passed"
else
    echo "❌ Host tests failed with exit code $HOST_EXIT_CODE"
fi

# Then run the eunice MCP server test
echo ""
echo "2. Running eunice MCP server test (test-container-eunice.sh)..."
./test-container-eunice.sh
EUNICE_EXIT_CODE=$?

if [ $EUNICE_EXIT_CODE -eq 0 ]; then
    echo "✅ Eunice MCP server test passed"
else
    echo "❌ Eunice MCP server test failed with exit code $EUNICE_EXIT_CODE"
fi

# Summary
echo ""
echo "=== Test Summary ==="
echo "Host tests: $([ $HOST_EXIT_CODE -eq 0 ] && echo "PASSED" || echo "FAILED")"
echo "Eunice MCP test: $([ $EUNICE_EXIT_CODE -eq 0 ] && echo "PASSED" || echo "FAILED")"

# Exit with failure if any test failed
if [ $HOST_EXIT_CODE -eq 0 ] && [ $EUNICE_EXIT_CODE -eq 0 ]; then
    echo "🎉 All container tests passed!"
    exit 0
else
    echo "💥 Some container tests failed!"
    exit 1
fi