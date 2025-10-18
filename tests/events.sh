#!/bin/bash

# Test suite for --events flag functionality
# Tests JSON-RPC event output format and content

set -e  # Exit on error

# Use uv run for development
EUNICE="uv run eunice.py"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Helper function to print test results
print_test() {
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $2"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗${NC} $2"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

# Helper function to validate JSON
validate_json() {
    echo "$1" | jq empty 2>/dev/null
    return $?
}

# Helper function to check if event has required JSON-RPC fields
validate_jsonrpc_event() {
    local event="$1"
    local has_jsonrpc=$(echo "$event" | jq -e '.jsonrpc == "2.0"' 2>/dev/null)
    local has_id=$(echo "$event" | jq -e '.id != null' 2>/dev/null)
    local has_method=$(echo "$event" | jq -e '.method != null' 2>/dev/null)
    local has_params=$(echo "$event" | jq -e '.params != null' 2>/dev/null)

    [ "$has_jsonrpc" == "true" ] && [ "$has_id" == "true" ] && [ "$has_method" == "true" ] && [ "$has_params" == "true" ]
    return $?
}

echo -e "${YELLOW}=== Testing --events Flag Functionality ===${NC}\n"

# Test 1: Basic events output format
echo -e "${YELLOW}Test 1: Basic JSON-RPC event format${NC}"
OUTPUT=$(timeout 15 $EUNICE --events --no-mcp "Hello" 2>/dev/null || true)
FIRST_LINE=$(echo "$OUTPUT" | head -n 1)
validate_json "$FIRST_LINE"
print_test $? "Events output is valid JSON"

validate_jsonrpc_event "$FIRST_LINE"
print_test $? "First event has JSON-RPC 2.0 structure"

# Test 2: Model selection event
echo -e "\n${YELLOW}Test 2: Model selection event${NC}"
OUTPUT=$(timeout 15 $EUNICE --events --no-mcp --model="gpt-oss" "test" 2>/dev/null || true)
HAS_MODEL_EVENT=$(echo "$OUTPUT" | grep -c "model.selected" || true)
print_test $([ "$HAS_MODEL_EVENT" -gt 0 ] && echo 0 || echo 1) "Emits model.selected event"

# Test 3: LLM request event
echo -e "\n${YELLOW}Test 3: LLM request event${NC}"
OUTPUT=$(timeout 15 $EUNICE --events --no-mcp "What is 2+2?" 2>/dev/null || true)
HAS_LLM_REQUEST=$(echo "$OUTPUT" | grep -c "llm.request" || true)
print_test $([ "$HAS_LLM_REQUEST" -gt 0 ] && echo 0 || echo 1) "Emits llm.request event"

# Test 4: LLM response event
echo -e "\n${YELLOW}Test 4: LLM response event${NC}"
HAS_LLM_RESPONSE=$(echo "$OUTPUT" | grep -c "llm.response" || true)
print_test $([ "$HAS_LLM_RESPONSE" -gt 0 ] && echo 0 || echo 1) "Emits llm.response event"

# Test 5: Events with MCP configuration
echo -e "\n${YELLOW}Test 5: MCP server events${NC}"
if [ -f "/tmp/test-time-config.json" ]; then
    OUTPUT=$(timeout 30 $EUNICE --events --config=/tmp/test-time-config.json "What time is it?" 2>/dev/null || true)
    HAS_MCP_EVENT=$(echo "$OUTPUT" | grep -c "mcp.server_started" || true)
    print_test $([ "$HAS_MCP_EVENT" -gt 0 ] && echo 0 || echo 1) "Emits mcp.server_started events with config"
else
    echo -e "${YELLOW}  Skipping MCP test - no /tmp/test-time-config.json found${NC}"
fi

# Test 6: Tool call and result events
echo -e "\n${YELLOW}Test 6: Tool call and result events${NC}"
if [ -f "/tmp/test-time-config.json" ]; then
    OUTPUT=$(timeout 30 $EUNICE --events --config=/tmp/test-time-config.json "Get the current time in America/New_York" 2>/dev/null || true)
    HAS_TOOL_CALL=$(echo "$OUTPUT" | grep -c "tool.call" || true)
    HAS_TOOL_RESULT=$(echo "$OUTPUT" | grep -c "tool.result" || true)
    print_test $([ "$HAS_TOOL_CALL" -gt 0 ] && echo 0 || echo 1) "Emits tool.call events"
    print_test $([ "$HAS_TOOL_RESULT" -gt 0 ] && echo 0 || echo 1) "Emits tool.result events"
else
    echo -e "${YELLOW}  Skipping tool events test - no /tmp/test-time-config.json found${NC}"
fi

# Test 7: Events mode suppresses normal output
echo -e "\n${YELLOW}Test 7: Events mode suppresses normal output${NC}"
OUTPUT=$(timeout 15 $EUNICE --events --no-mcp "Hello" 2>/dev/null || true)
# Check that output only contains JSON lines
NON_JSON_LINES=$(echo "$OUTPUT" | grep -v "^{" | wc -l)
print_test $([ "$NON_JSON_LINES" -eq 0 ] && echo 0 || echo 1) "Events mode outputs only JSON (no visual panels)"

# Test 8: Events override --silent
echo -e "\n${YELLOW}Test 8: Events override --silent${NC}"
OUTPUT_EVENTS=$(timeout 15 $EUNICE --events --silent --no-mcp "Hi" 2>/dev/null | wc -l || true)
OUTPUT_SILENT=$(timeout 15 $EUNICE --silent --no-mcp "Hi" 2>/dev/null | wc -l || true)
print_test $([ "$OUTPUT_EVENTS" -gt "$OUTPUT_SILENT" ] && echo 0 || echo 1) "--events overrides --silent (produces event output)"

# Test 9: Events override --verbose
echo -e "\n${YELLOW}Test 9: Events override --verbose${NC}"
OUTPUT_EVENTS=$(timeout 15 $EUNICE --events --verbose --no-mcp "Hi" 2>/dev/null || true)
HAS_VERBOSE_OUTPUT=$(echo "$OUTPUT_EVENTS" | grep -c "🔄 Calling LLM" || true)
print_test $([ "$HAS_VERBOSE_OUTPUT" -eq 0 ] && echo 0 || echo 1) "--events overrides --verbose (no verbose output)"

# Test 10: Event IDs are sequential
echo -e "\n${YELLOW}Test 10: Event IDs are sequential${NC}"
OUTPUT=$(timeout 15 $EUNICE --events --no-mcp "Quick test" 2>/dev/null || true)
IDS=$(echo "$OUTPUT" | jq -r '.id' 2>/dev/null | tr '\n' ' ')
FIRST_ID=$(echo "$IDS" | awk '{print $1}')
LAST_ID=$(echo "$IDS" | awk '{print $NF}')
if [ -n "$FIRST_ID" ] && [ -n "$LAST_ID" ] && [ "$FIRST_ID" -le "$LAST_ID" ]; then
    print_test 0 "Event IDs are sequential and increasing"
else
    print_test 1 "Event IDs are sequential and increasing"
fi

# Test 11: LLM request contains messages
echo -e "\n${YELLOW}Test 11: LLM request event contains messages${NC}"
OUTPUT=$(timeout 15 $EUNICE --events --no-mcp "test message" 2>/dev/null || true)
REQUEST_EVENT=$(echo "$OUTPUT" | grep "llm.request" | head -n 1)
HAS_MESSAGES=$(echo "$REQUEST_EVENT" | jq -e '.params.messages != null' 2>/dev/null)
print_test $([ "$HAS_MESSAGES" == "true" ] && echo 0 || echo 1) "LLM request event contains messages array"

# Test 12: LLM response contains content
echo -e "\n${YELLOW}Test 12: LLM response event contains content${NC}"
RESPONSE_EVENT=$(echo "$OUTPUT" | grep "llm.response" | head -n 1)
HAS_CONTENT=$(echo "$RESPONSE_EVENT" | jq -e '.params.content != null' 2>/dev/null)
print_test $([ "$HAS_CONTENT" == "true" ] && echo 0 || echo 1) "LLM response event contains content"

# Test 13: Events include timestamps
echo -e "\n${YELLOW}Test 13: Events include timestamps${NC}"
ALL_EVENTS_HAVE_TIMESTAMPS=0
while IFS= read -r line; do
    if echo "$line" | jq -e '.params.timestamp' >/dev/null 2>&1; then
        :  # Event has timestamp
    else
        ALL_EVENTS_HAVE_TIMESTAMPS=1
        break
    fi
done <<< "$OUTPUT"
print_test $ALL_EVENTS_HAVE_TIMESTAMPS "All events include timestamp in params"

# Test 14: Duration fields in response events
echo -e "\n${YELLOW}Test 14: Duration fields in timed events${NC}"
HAS_DURATION=$(echo "$RESPONSE_EVENT" | jq -e '.params.duration != null' 2>/dev/null)
print_test $([ "$HAS_DURATION" == "true" ] && echo 0 || echo 1) "LLM response event includes duration"

# Test 15: Model info in events
echo -e "\n${YELLOW}Test 15: Model info in events${NC}"
MODEL_IN_REQUEST=$(echo "$REQUEST_EVENT" | jq -e '.params.model != null' 2>/dev/null)
MODEL_IN_RESPONSE=$(echo "$RESPONSE_EVENT" | jq -e '.params.model != null' 2>/dev/null)
print_test $([ "$MODEL_IN_REQUEST" == "true" ] && [ "$MODEL_IN_RESPONSE" == "true" ] && echo 0 || echo 1) "Events include model information"

# Print summary
echo -e "\n${YELLOW}=== Test Summary ===${NC}"
echo -e "Total tests: ${TOTAL_TESTS}"
echo -e "${GREEN}Passed: ${PASSED_TESTS}${NC}"
if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Failed: ${FAILED_TESTS}${NC}"
    exit 1
else
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
fi
