# Context Compression Feature Design

## Overview

This document describes the context compression feature for eunice, which automatically handles Gemini's "resource exhausted" errors by summarizing conversation history and continuing with compressed context.

## Problem Statement

When using Gemini models (especially `gemini-2.0-flash` or `gemini-2.5-pro`), long conversations can trigger a `RESOURCE_EXHAUSTED` error when the context window is exceeded. Currently, this causes the agent to fail. Instead, we should:

1. Detect the error
2. Compress the conversation history
3. Retry with the compressed context

## Research: How Other Tools Handle This

Based on research into [Claude Code](https://stevekinney.com/courses/ai-development/claude-code-compaction), [OpenAI Codex CLI](https://gist.github.com/badlogic/cd2ef65b0697c4dbe2d13fbecb0a0a5f), and [Anthropic's engineering guidance](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents):

### Claude Code Approach
- **Trigger**: Manual `/compact` or automatic at ~95% context capacity
- **Strategy**: Generate LLM summary of entire conversation, start new session with summary as initial context
- **Key insight**: Preserve architectural decisions, unresolved bugs, implementation details; discard redundant tool outputs

### OpenAI Codex CLI Approach
- **Trigger**: Token-based threshold (180k-244k tokens)
- **Strategy**: Full history → summarization prompt → rebuild as: initial context + recent messages (~20k tokens) + summary
- **Key insight**: Prefix summary with explanation that "another model created this checkpoint"

### Anthropic's Recommended Techniques
1. **Compaction**: Summarize conversations, preserve critical decisions
2. **Tool result clearing**: Remove redundant tool outputs (lightweight compaction)
3. **Sub-agent architectures**: Delegate to focused agents, return condensed summaries

## Design for Eunice

### 1. Error Detection

Detect Gemini's resource exhausted error in `src/client.rs`:

```rust
// In chat_completion() response handling
if let Some(error) = &response.error {
    if error.message.contains("RESOURCE_EXHAUSTED")
       || error.message.contains("context length")
       || error.code == 429 && error.message.contains("quota") {
        return Err(ContextExhaustedError {
            message: error.message.clone(),
            messages: messages.clone()
        });
    }
}
```

### 2. Compression Strategy

When context is exhausted, compress using a two-phase approach:

**Phase 1: Tool Output Clearing**
- Remove raw tool outputs older than the last N messages (keeping the last 5-10)
- Replace with compact summaries: `[Tool: shell_exec returned 2847 chars, exit code 0]`
- This is "lightweight compaction" - often sufficient to continue

**Phase 2: Full Summarization (if Phase 1 insufficient)**
- Send conversation to model with summarization prompt
- Create compressed context preserving critical information

### 3. Summarization Prompt

```markdown
## Context Compression Task

Your task is to create a detailed summary of the conversation so far. This summary will be used as context when continuing the conversation, so preserve ALL critical information.

### What to Preserve
- **User's original request**: The task they asked for
- **What was accomplished**: Files created/modified, commands run, decisions made
- **Current work in progress**: Any incomplete tasks
- **Files involved**: List of files read, written, or modified with brief descriptions
- **Tool results summary**: Key outputs from tool calls (errors, important data)
- **Next steps**: What remains to be done
- **Key constraints**: Any user preferences, requirements, or limitations mentioned
- **Unresolved issues**: Errors, bugs, or blockers encountered

### What to Discard
- Redundant tool outputs (keep summaries only)
- Verbose file contents (keep filenames and key snippets)
- Repetitive conversation turns
- Debug output no longer relevant

### Format
Produce a structured summary that another instance of this model can use to seamlessly continue the conversation. Start with:

```
## Conversation Summary (Compressed Context)

**Original Task**: [one-line description]

**Completed**:
- [item 1]
- [item 2]

**In Progress**:
- [current task]

**Files**:
- `path/to/file.rs`: [brief description of changes]

**Key Decisions**:
- [decision 1 and rationale]

**Next Steps**:
1. [step 1]
2. [step 2]

**Notes**:
- [any important context]
```
```

### 4. Implementation Location

New module: `src/compress.rs`

```rust
use crate::models::{Message, ToolCall};
use crate::client::Client;

/// Compression configuration
pub struct CompressionConfig {
    /// Keep last N messages with full tool outputs
    pub preserve_recent_messages: usize,  // default: 10
    /// Maximum tokens for compressed context
    pub target_tokens: usize,  // default: 8000
    /// Whether to attempt lightweight compaction first
    pub try_lightweight_first: bool,  // default: true
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            preserve_recent_messages: 10,
            target_tokens: 8000,
            try_lightweight_first: true,
        }
    }
}

/// Result of compression
pub struct CompressedContext {
    pub summary: String,
    pub preserved_messages: Vec<Message>,
    pub compression_ratio: f32,
}

/// Compress conversation history
pub async fn compress_context(
    client: &Client,
    messages: &[Message],
    system_prompt: Option<&str>,
    config: &CompressionConfig,
) -> Result<CompressedContext> {
    // Phase 1: Try lightweight compaction (tool output clearing)
    if config.try_lightweight_first {
        let lightweight = lightweight_compact(messages, config);
        let token_estimate = estimate_tokens(&lightweight);

        if token_estimate < config.target_tokens {
            return Ok(CompressedContext {
                summary: String::new(),
                preserved_messages: lightweight,
                compression_ratio: token_estimate as f32 / estimate_tokens(messages) as f32,
            });
        }
    }

    // Phase 2: Full summarization
    let summary = generate_summary(client, messages, system_prompt).await?;

    // Combine: summary + recent messages
    let recent = &messages[messages.len().saturating_sub(config.preserve_recent_messages)..];

    Ok(CompressedContext {
        summary,
        preserved_messages: recent.to_vec(),
        compression_ratio: /* calculate */,
    })
}

/// Lightweight compaction: clear old tool outputs
fn lightweight_compact(messages: &[Message], config: &CompressionConfig) -> Vec<Message> {
    let cutoff = messages.len().saturating_sub(config.preserve_recent_messages);

    messages.iter().enumerate().map(|(i, msg)| {
        if i < cutoff {
            // Compress old tool outputs
            match msg {
                Message::Tool { id, content } => {
                    let summary = if content.len() > 200 {
                        format!("[Tool output: {} chars]", content.len())
                    } else {
                        content.clone()
                    };
                    Message::Tool { id: id.clone(), content: summary }
                }
                _ => msg.clone()
            }
        } else {
            msg.clone()
        }
    }).collect()
}

/// Generate full summary using the model
async fn generate_summary(
    client: &Client,
    messages: &[Message],
    system_prompt: Option<&str>,
) -> Result<String> {
    let compression_prompt = include_str!("../compression_prompt.md");

    // Format conversation for summarization
    let conversation_text = format_conversation_for_summary(messages);

    let summary_request = vec![
        Message::User {
            content: format!(
                "{}\n\n## Conversation to Summarize\n\n{}",
                compression_prompt,
                conversation_text
            )
        }
    ];

    let response = client.chat_completion(&summary_request, &[], None).await?;

    Ok(response.choices[0].message.content.clone())
}
```

### 5. Integration into Agent Loop

In `src/agent.rs`, wrap the main completion call:

```rust
pub async fn run_agent_loop(/* ... */) -> Result<()> {
    let compression_config = CompressionConfig::default();

    loop {
        match client.chat_completion(&messages, &tools, system_prompt.as_deref()).await {
            Ok(response) => {
                // Normal processing...
            }
            Err(e) if e.is_context_exhausted() => {
                eprintln!("{}  Context exhausted, compressing...", spinner_chars());

                let compressed = compress_context(
                    &client,
                    &messages,
                    system_prompt.as_deref(),
                    &compression_config
                ).await?;

                // Rebuild message history
                messages = if compressed.summary.is_empty() {
                    // Lightweight compaction was sufficient
                    compressed.preserved_messages
                } else {
                    // Full summarization: start with summary + recent messages
                    let mut new_messages = vec![
                        Message::User {
                            content: format!(
                                "## Continuing from compressed context\n\n{}\n\n---\n\nPlease continue with the task.",
                                compressed.summary
                            )
                        }
                    ];
                    new_messages.extend(compressed.preserved_messages);
                    new_messages
                };

                eprintln!("  Compressed to {:.0}% of original size",
                    compressed.compression_ratio * 100.0);

                // Retry with compressed context
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 6. CLI Options

Add flags to control compression behavior:

```rust
/// Enable/disable auto-compression (default: enabled in DMN mode)
#[arg(long)]
pub auto_compress: Option<bool>,

/// Target token count after compression
#[arg(long, default_value = "8000")]
pub compress_target_tokens: usize,

/// Number of recent messages to preserve in full
#[arg(long, default_value = "10")]
pub compress_preserve_recent: usize,
```

### 7. Config File Support

In `eunice.toml`:

```toml
[compression]
enabled = true
target_tokens = 8000
preserve_recent = 10
try_lightweight_first = true
```

## Token Estimation

Simple heuristic for token estimation (no external dependency):

```rust
/// Rough token estimate: ~4 chars per token for English
fn estimate_tokens(messages: &[Message]) -> usize {
    messages.iter().map(|m| {
        match m {
            Message::User { content } => content.len() / 4,
            Message::Assistant { content, tool_calls } => {
                content.as_ref().map(|c| c.len()).unwrap_or(0) / 4
                + tool_calls.as_ref().map(|tc| tc.len() * 50).unwrap_or(0)
            }
            Message::Tool { content, .. } => content.len() / 4,
        }
    }).sum()
}
```

## Testing Strategy

1. **Unit tests**: Test compression functions with mock conversations
2. **Integration test**: Create a conversation that exceeds context, verify compression works
3. **Manual test**: Run long DMN session, observe compression behavior

## Future Enhancements

1. **Proactive compression**: Compress at 85% capacity before hitting limits
2. **Incremental summaries**: Generate rolling summaries periodically
3. **Memory persistence**: Save summaries to disk for session resume
4. **Multi-model compression**: Use smaller/faster model for summarization
5. **Selective preservation**: User-defined rules for what to preserve

## References

- [Claude Code Compaction](https://stevekinney.com/courses/ai-development/claude-code-compaction)
- [Context Compaction Research](https://gist.github.com/badlogic/cd2ef65b0697c4dbe2d13fbecb0a0a5f)
- [Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Forge Code Context Compaction](https://forgecode.dev/docs/context-compaction/)
- [Claude Auto-Compact Guide](https://www.arsturn.com/blog/why-does-claude-forget-things-understanding-auto-compact-context-windows)
