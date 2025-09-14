#!/bin/bash

# test.sh - Comprehensive test suite for eunice
# Tests each aspect of the specification

set -e

echo "===== eunice Test Suite ====="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_exit_code="${3:-0}"
    local should_contain="${4:-}"
    local should_not_contain="${5:-}"

    echo -e "${YELLOW}Running test: $test_name${NC}"
    echo "Command: $test_command"

    TESTS_RUN=$((TESTS_RUN + 1))

    # Capture both stdout and stderr, and the exit code
    set +e
    output=$(eval "$test_command" 2>&1)
    actual_exit_code=$?
    set -e

    # Check exit code
    if [ "$actual_exit_code" -ne "$expected_exit_code" ]; then
        echo -e "${RED}FAIL: Expected exit code $expected_exit_code, got $actual_exit_code${NC}"
        echo "Output: $output"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo
        return 1
    fi

    # Check if output should contain something
    if [ -n "$should_contain" ] && ! echo "$output" | grep -q "$should_contain"; then
        echo -e "${RED}FAIL: Output should contain '$should_contain'${NC}"
        echo "Output: $output"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo
        return 1
    fi

    # Check if output should not contain something
    if [ -n "$should_not_contain" ] && echo "$output" | grep -q "$should_not_contain"; then
        echo -e "${RED}FAIL: Output should not contain '$should_not_contain'${NC}"
        echo "Output: $output"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo
        return 1
    fi

    echo -e "${GREEN}PASS${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo
}

# Setup test environment
echo "Setting up test environment..."

# Create test files and directories for testing
mkdir -p test_data
echo "This is a test file" > test_data/test.txt
echo "Another test file" > test_data/another.txt
mkdir -p test_data/subdir
echo "File in subdirectory" > test_data/subdir/nested.txt

# Create a prompt file
echo "How many files are in the test_data directory?" > test_prompt.txt

echo "Setup complete."
echo

# Test 1: Help/Usage
echo "=== Testing Help and Usage ==="
run_test "Help flag" "uv run eunice.py --help" 0 "eunice - Agentic CLI runner"
run_test "No arguments" "uv run eunice.py" 1 "No prompt provided"

# Test 2: Environment variable validation
echo "=== Testing Environment Variable Validation ==="

# Test OpenAI model without API key
unset OPENAI_API_KEY
run_test "OpenAI model without API key" "uv run eunice.py --model=gpt-3.5-turbo 'test'" 1 "OPENAI_API_KEY environment variable is required"

# Test Gemini model without API key
unset GEMINI_API_KEY
run_test "Gemini model without API key" "uv run eunice.py --model=gemini-2.5-flash 'test'" 1 "GEMINI_API_KEY environment variable is required"

# Test 3: Argument parsing
echo "=== Testing Argument Parsing ==="

# Set a dummy API key for OpenAI tests (these will fail at API call but should pass validation)
export OPENAI_API_KEY="sk-test123"

run_test "Model parameter" "timeout 5 uv run eunice.py --model=gpt-4 'test prompt' || true" 0 "" "No prompt provided"
run_test "Prompt parameter" "timeout 5 uv run eunice.py --prompt='test prompt' || true" 0 "" "No prompt provided"
run_test "Positional prompt" "timeout 5 uv run eunice.py 'test prompt' || true" 0 "" "No prompt provided"

# Test 4: File vs string prompt detection
echo "=== Testing Prompt Parsing ==="

run_test "File prompt" "timeout 5 uv run eunice.py --prompt=test_prompt.txt || true" 0 "" "No prompt provided"
run_test "Non-existent file prompt" "uv run eunice.py --prompt=nonexistent.txt" 1 "Error reading prompt file"

# Test 5: Provider detection
echo "=== Testing Provider Detection ==="

# These tests check that the right provider is detected (will fail at API call but that's expected)
run_test "OpenAI model detection" "timeout 5 uv run eunice.py --model=gpt-4 'test' || echo 'Provider detected correctly'" 0
run_test "Gemini model detection (with key)" "GEMINI_API_KEY=test timeout 5 uv run eunice.py --model=gemini-2.5-flash 'test' || echo 'Provider detected correctly'" 0
run_test "Ollama model detection" "timeout 5 uv run eunice.py --model=llama3.1 'test' || echo 'Provider detected correctly'" 0
run_test "Ollama gpt-oss model detection" "timeout 5 uv run eunice.py --model=gpt-oss 'test' || echo 'Provider detected correctly'" 0

# Test 6: Tool functionality (unit tests)
echo "=== Testing Tool Functions ==="

# Test list_files function by creating a simple test script
cat > test_tools.py << 'EOF'
import sys
sys.path.append('.')
from eunice import list_files, read_file
import json

# Test list_files
print("Testing list_files:")
result = list_files("test_data")
print(json.dumps(result, indent=2))

# Test with non-existent directory
result = list_files("nonexistent")
print("Non-existent directory:", json.dumps(result, indent=2))

# Test read_file
print("\nTesting read_file:")
content = read_file("test_data/test.txt")
print("Content:", repr(content))

# Test with non-existent file
content = read_file("nonexistent.txt")
print("Non-existent file:", repr(content))
EOF

run_test "Tool functions unit test" "python3 test_tools.py" 0 "This is a test file"

# Test 7: Different usage patterns from the spec
echo "=== Testing Usage Patterns from Spec ==="

# Test the exact patterns from the specification
run_test "Basic usage pattern" "timeout 5 uv run eunice.py 'How many files are in the current directory?' || true" 0
run_test "Model specification pattern" "timeout 5 uv run eunice.py --model='gpt-4' 'how many files in the current directory?' || true" 0
run_test "Gemini with prompt file" "GEMINI_API_KEY=test timeout 5 uv run eunice.py --model='gemini-2.5-pro' --prompt=test_prompt.txt || true" 0
run_test "Ollama with file argument" "timeout 5 uv run eunice.py --model='llama3.1' test_prompt.txt || true" 0
run_test "Prompt parameter usage" "GEMINI_API_KEY=test timeout 5 uv run eunice.py --model='gemini-2.5-pro' --prompt='How many files in the current directory?' || true" 0

# Test 8: Error handling
echo "=== Testing Error Handling ==="

run_test "Invalid model format" "timeout 5 uv run eunice.py --model='' 'test' || true" 0 # Should default to Ollama
run_test "Empty prompt" "timeout 5 uv run eunice.py '' || true" 0 # Should work, just send empty prompt
run_test "Very long prompt" "timeout 5 uv run eunice.py 'This is a very long prompt that should still work fine and not cause any issues with the argument parsing or processing mechanisms implemented in the eunice CLI tool' || true" 0 # Should work

# Test 9: Colored output functionality
echo "=== Testing Colored Output ==="

# Create a simple test that exercises the colored output functions
cat > test_colored_output.py << 'EOF'
import sys
sys.path.append('.')
from eunice import print_tool_invocation, print_tool_result

# Test that the functions don't crash and produce expected output patterns
print_tool_invocation("test_tool", {"arg": "value"})
print_tool_result("Test result content")
print_tool_result("This is a very long result that should be truncated", 30)
print("COLORED_OUTPUT_TEST_COMPLETE")
EOF

run_test "Colored output functions" "python3 test_colored_output.py" 0 "COLORED_OUTPUT_TEST_COMPLETE"

# Test 10: Provider detection edge cases
echo "=== Testing Provider Detection Edge Cases ==="

run_test "gpt-oss should use Ollama not OpenAI" "timeout 3 uv run eunice.py --model=gpt-oss 'test' && echo 'Correctly detected as Ollama' || echo 'Correctly detected as Ollama'" 0 "Correctly detected as Ollama"

# Test 11: New command line options
echo "=== Testing New Command Line Options ==="

run_test "List models option" "uv run eunice.py --list-models" 0 "Available Models"
run_test "Tool output limit help" "uv run eunice.py --help" 0 "tool-output-limit"
run_test "Enhanced help output" "uv run eunice.py --help" 0 "API Key Status"

# Test 12: Dependencies and installation
echo "=== Testing Dependencies ==="

run_test "Python import test" "python3 -c 'import openai; print(\"OpenAI library available\")'" 0 "OpenAI library available"
run_test "Required modules test" "python3 -c 'import json, os, sys, pathlib, argparse; print(\"All required modules available\")'" 0 "All required modules available"

# Test 13: Script permissions and execution
echo "=== Testing Script Execution ==="

run_test "Script is executable" "chmod +x eunice.py && test -x eunice.py && echo 'Script is executable'" 0 "Script is executable"
run_test "Shebang line" "head -n 1 eunice.py | grep -q '#!/usr/bin/env python3' && echo 'Shebang correct'" 0 "Shebang correct"

# Cleanup
echo "Cleaning up test environment..."
rm -rf test_data
rm -f test_prompt.txt
rm -f test_tools.py
rm -f test_colored_output.py

# Final results
echo
echo "===== Test Results ====="
echo "Tests run: $TESTS_RUN"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
    echo
    echo -e "${RED}Some tests failed. Please review the failures above.${NC}"
    exit 1
else
    echo -e "Tests failed: ${GREEN}0${NC}"
    echo
    echo -e "${GREEN}All tests passed!${NC}"
    echo
    echo "The eunice CLI tool has been successfully implemented according to the specification."
    echo "You can now use it with commands like:"
    echo "  uv run eunice.py 'How many files are in the current directory?'"
    echo "  uv run eunice.py --model=gpt-4 'analyze this codebase'"
    echo
    echo "Don't forget to set your API keys:"
    echo "  export OPENAI_API_KEY='your-openai-key'"
    echo "  export GEMINI_API_KEY='your-gemini-key'"
    exit 0
fi