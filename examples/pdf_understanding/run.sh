#!/bin/bash
# Test PDF understanding with eunice

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

# Run eunice in DMN mode with the instructions
"$EUNICE_PATH" --dmn --prompt instructions.md
