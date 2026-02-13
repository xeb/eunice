# Context Compaction Test

This example demonstrates and tests the automatic context compaction feature in Eunice.

## How Context Compaction Works

When the model returns a context exhaustion error (e.g., Gemini's `RESOURCE_EXHAUSTED`), Eunice automatically:

1. **Lightweight Compaction** (tried first): Truncates old tool outputs while preserving recent messages
2. **Full Summarization** (if needed): Uses the LLM to generate a summary of the conversation

## Testing Compaction

### Manual Test

To test context compaction, you need to exhaust the context window. This typically happens with:

- Long conversations with many tool calls
- Large file reads
- Extensive code analysis

```bash
# Start an interactive session with a task that will exhaust context
eunice --chat
> Read every file in this large project and analyze them all
```

When context is exhausted, you'll see:
```
⚠️  Context exhausted. Compacting conversation history...
✓ Compacted to 45% of original size using lightweight compaction
```

### Unit Tests

Run the test script to verify the compaction logic:

```bash
./test_compaction.sh
```

This script tests the `is_context_exhausted_error` detection and runs `cargo test` for compaction unit tests.

## Error Detection

Eunice detects context exhaustion from various providers:

- **Gemini**: `RESOURCE_EXHAUSTED`, `resource exhausted`
- **OpenAI**: `context_length_exceeded`, `maximum context length`
- **Anthropic**: `prompt is too long`
- **Generic**: `token` + `limit`, `context` + `exceed`
