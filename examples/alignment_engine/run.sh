#!/bin/bash

# Default to 1 iteration
n=1

# Parse arguments
for arg in "$@"; do
    case $arg in
        --n=*)
            n="${arg#*=}"
            shift
            ;;
    esac
done

# Run the command n times
for ((i=1; i<=n; i++)); do
    echo "Run $i of $n"
    eunice --dmn --model=gemini-3-pro-preview instructions.md
done
