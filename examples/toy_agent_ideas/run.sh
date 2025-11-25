#!/bin/bash

# Defaults
n=1
model="gemini-3-pro-preview"

# Parse arguments
for arg in "$@"; do
    case $arg in
        --n=*)
            n="${arg#*=}"
            shift
            ;;
        --model=*)
            model="${arg#*=}"
            shift
            ;;
    esac
done

# Run the command n times
for ((i=1; i<=n; i++)); do
    echo "Run $i of $n"
    eunice --dmn --model="$model" instructions.md
done
