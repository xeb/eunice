#!/bin/bash
# run.sh - Run the websearch example

# Ensure eunice is in the PATH or use a relative path
if ! command -v eunice &> /dev/null
then
    # Assuming the binary is in target/release relative to project root
    EUNICE_PATH="../../target/release/eunice"
    if [ ! -f "$EUNICE_PATH" ]; then
        echo "eunice binary not found. Please build the project first with 'cargo build --release'"
        exit 1
    fi
else
    EUNICE_PATH="eunice"
fi

# Check for GEMINI_API_KEY
if [ -z "$GEMINI_API_KEY" ]; then
    echo "Error: GEMINI_API_KEY environment variable not set"
    echo "The search_query tool requires a Gemini API key."
    exit 1
fi

# Run eunice in DMN mode with the instructions
"$EUNICE_PATH" --dmn --prompt instructions.md
