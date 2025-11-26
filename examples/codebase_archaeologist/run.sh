#!/bin/bash

# Codebase Archaeologist - Autonomous codebase exploration agent
# Usage: ./run.sh [--n=<iterations>] [--model=<model>] [--target=<path>]

# Defaults
n=1
model="gemini-3-pro-preview"
target=""

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
        --target=*)
            target="${arg#*=}"
            shift
            ;;
    esac
done

# If target specified, write it to workspace/target.txt
if [ -n "$target" ]; then
    mkdir -p workspace
    echo "$target" > workspace/target.txt
    echo "Target set to: $target"
fi

# Run the agent n times (each run explores one component)
for ((i=1; i<=n; i++)); do
    echo "=== Exploration Run $i of $n ==="
    eunice --dmn --model="$model" instructions.md
    echo ""
done
