#!/bin/bash

# Codebase Archaeologist - Autonomous codebase exploration agent
# Usage: ./run.sh [--target=<path>]

target=""

# Parse arguments
for arg in "$@"; do
    case $arg in
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

# Run the exploration
eunice --prompt instructions.md "Explore the codebase and document your findings."
