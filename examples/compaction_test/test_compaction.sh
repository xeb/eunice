#!/bin/bash
# Test script for context compaction feature

set -e

echo "=== Context Compaction Test ==="
echo

# Navigate to project root
cd "$(dirname "$0")/../.."

echo "1. Running unit tests for compaction module..."
cargo test compact:: --quiet
echo "   ✓ Unit tests passed"
echo

echo "2. Testing error detection patterns..."
cargo test is_context_exhausted_error --quiet
echo "   ✓ Error detection tests passed"
echo

echo "3. Testing lightweight compaction..."
cargo test lightweight_compact --quiet
echo "   ✓ Lightweight compaction tests passed"
echo

echo "4. Testing token estimation..."
cargo test estimate_tokens --quiet
echo "   ✓ Token estimation tests passed"
echo

echo "=== All compaction tests passed! ==="
echo
echo "To test live compaction, start a chat session:"
echo "  eunice --chat"
echo "  > <task that generates lots of tool output>"
echo
echo "When context exhausts, you'll see:"
echo "  ⚠️  Context exhausted. Compacting conversation history..."
echo "  ✓ Compacted to XX% of original size using [method]"
