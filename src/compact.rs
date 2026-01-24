//! Context compaction for handling context window exhaustion
//!
//! This module provides functionality to compact conversation history when
//! the context window is exhausted (e.g., Gemini's RESOURCE_EXHAUSTED error).
//!
//! Two-phase compaction strategy:
//! 1. Lightweight compaction: Clear old tool outputs (often sufficient)
//! 2. Full summarization: LLM-generated summary of the conversation

use crate::client::Client;
use crate::models::Message;
use anyhow::{Context, Result};

/// Compaction configuration
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Keep last N messages with full tool outputs
    pub preserve_recent_messages: usize,
    /// Maximum characters for compacted tool outputs in lightweight mode
    pub tool_output_max_chars: usize,
    /// Whether to attempt lightweight compaction first
    pub try_lightweight_first: bool,
    /// Enable compaction (can be disabled via config)
    pub enabled: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            preserve_recent_messages: 10,
            tool_output_max_chars: 200,
            try_lightweight_first: true,
            enabled: true,
        }
    }
}

/// Result of compaction
#[derive(Debug)]
#[allow(dead_code)]
pub struct CompactedContext {
    /// Summary of the conversation (empty if lightweight compaction was sufficient)
    pub summary: String,
    /// Messages to use going forward
    pub messages: Vec<Message>,
    /// Compaction ratio (compacted / original)
    pub compaction_ratio: f32,
    /// Whether full summarization was used (vs lightweight)
    pub used_full_summarization: bool,
}

/// Compaction prompt template (embedded from file)
const COMPACTION_PROMPT: &str = include_str!("../prompts/compaction_prompt.md");

/// Estimate token count for messages (rough heuristic: ~4 chars per token)
pub fn estimate_tokens(messages: &[Message]) -> usize {
    messages
        .iter()
        .map(|m| match m {
            Message::User { content } => content.len() / 4,
            Message::Assistant { content, tool_calls } => {
                let content_tokens = content.as_ref().map(|c| c.len()).unwrap_or(0) / 4;
                let tool_tokens = tool_calls
                    .as_ref()
                    .map(|tc| tc.iter().map(|t| t.function.arguments.len() / 4 + 50).sum())
                    .unwrap_or(0);
                content_tokens + tool_tokens
            }
            Message::Tool { content, .. } => content.len() / 4,
        })
        .sum()
}

/// Perform lightweight compaction by truncating old tool outputs
fn lightweight_compact(messages: &[Message], config: &CompactionConfig) -> Vec<Message> {
    let cutoff = messages.len().saturating_sub(config.preserve_recent_messages);

    messages
        .iter()
        .enumerate()
        .map(|(i, msg)| {
            if i < cutoff {
                // Compact old tool outputs
                match msg {
                    Message::Tool {
                        tool_call_id,
                        content,
                    } => {
                        let summary = if content.len() > config.tool_output_max_chars {
                            // Extract first and last lines for context
                            let lines: Vec<&str> = content.lines().collect();
                            if lines.len() > 4 {
                                format!(
                                    "[Tool output truncated: {} chars, {} lines]\n{}\n...\n{}",
                                    content.len(),
                                    lines.len(),
                                    lines[..2].join("\n"),
                                    lines[lines.len() - 2..].join("\n")
                                )
                            } else {
                                format!("[Tool output: {} chars]", content.len())
                            }
                        } else {
                            content.clone()
                        };
                        Message::Tool {
                            tool_call_id: tool_call_id.clone(),
                            content: summary,
                        }
                    }
                    _ => msg.clone(),
                }
            } else {
                msg.clone()
            }
        })
        .collect()
}

/// Format conversation history for summarization
fn format_conversation_for_summary(messages: &[Message]) -> String {
    messages
        .iter()
        .enumerate()
        .map(|(i, msg)| match msg {
            Message::User { content } => format!("[{}] USER:\n{}\n", i, content),
            Message::Assistant { content, tool_calls } => {
                let content_str = content.as_deref().unwrap_or("");
                let tools_str = tool_calls
                    .as_ref()
                    .map(|tc| {
                        tc.iter()
                            .map(|t| format!("  -> {}({})", t.function.name, truncate(&t.function.arguments, 100)))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                if tools_str.is_empty() {
                    format!("[{}] ASSISTANT:\n{}\n", i, content_str)
                } else {
                    format!("[{}] ASSISTANT:\n{}\nTool calls:\n{}\n", i, content_str, tools_str)
                }
            }
            Message::Tool {
                tool_call_id,
                content,
            } => format!(
                "[{}] TOOL RESULT ({}):\n{}\n",
                i,
                tool_call_id,
                truncate(content, 500)
            ),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Truncate a string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...[truncated]", &s[..max_len])
    }
}

/// Generate a full summary using the LLM
async fn generate_summary(
    client: &Client,
    model: &str,
    messages: &[Message],
) -> Result<String> {
    let conversation_text = format_conversation_for_summary(messages);

    let summary_prompt = format!("{}\n\n{}", COMPACTION_PROMPT, conversation_text);

    // Create a simple request for summarization (no tools)
    let summary_messages = vec![Message::User {
        content: summary_prompt,
    }];

    let response = client
        .chat_completion(
            model,
            serde_json::to_value(&summary_messages)?,
            None, // No tools for summarization
        )
        .await
        .context("Failed to generate context summary")?;

    let content = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Failed to generate summary".to_string());

    Ok(content)
}

/// Maximum messages before forcing hard trim (full summarization would also exceed context)
const HARD_TRIM_THRESHOLD: usize = 500;

/// Number of recent messages to keep during hard trim
const HARD_TRIM_KEEP: usize = 50;

/// Compact conversation history
///
/// This function implements a three-phase compaction strategy:
/// 1. Hard trim: If message count is very large, keep only recent messages
/// 2. Lightweight compaction: Clear old tool outputs (often sufficient)
/// 3. Full summarization: LLM-generated summary of the conversation
pub async fn compact_context(
    client: &Client,
    model: &str,
    messages: &[Message],
    config: &CompactionConfig,
) -> Result<CompactedContext> {
    if messages.is_empty() {
        return Ok(CompactedContext {
            summary: String::new(),
            messages: vec![],
            compaction_ratio: 1.0,
            used_full_summarization: false,
        });
    }

    let original_tokens = estimate_tokens(messages);

    // Phase 0: Hard trim for extremely large histories
    // With thousands of messages, even summarization would exceed context limits
    if messages.len() > HARD_TRIM_THRESHOLD {
        let keep_start = messages.len().saturating_sub(HARD_TRIM_KEEP);
        let mut trimmed = vec![Message::User {
            content: format!(
                "## Context Note\n\n[Previous conversation history ({} messages) was trimmed to stay within token limits. The most recent {} messages are preserved below.]",
                messages.len(),
                HARD_TRIM_KEEP
            ),
        }];
        trimmed.extend(messages[keep_start..].to_vec());

        // Also apply lightweight compaction to the trimmed messages
        let compacted = lightweight_compact(&trimmed, config);
        let new_tokens = estimate_tokens(&compacted);
        let ratio = new_tokens as f32 / original_tokens as f32;

        return Ok(CompactedContext {
            summary: String::new(),
            messages: compacted,
            compaction_ratio: ratio,
            used_full_summarization: false,
        });
    }

    // Phase 1: Try lightweight compaction
    if config.try_lightweight_first {
        let lightweight = lightweight_compact(messages, config);
        let lightweight_tokens = estimate_tokens(&lightweight);

        // If we achieved significant compaction (>30% reduction), use it
        let ratio = lightweight_tokens as f32 / original_tokens as f32;
        if ratio < 0.7 {
            return Ok(CompactedContext {
                summary: String::new(),
                messages: lightweight,
                compaction_ratio: ratio,
                used_full_summarization: false,
            });
        }
    }

    // Phase 2: Full summarization
    let summary = generate_summary(client, model, messages).await?;

    // Create new message history: summary as context + recent messages
    let recent_start = messages.len().saturating_sub(config.preserve_recent_messages);
    let recent_messages: Vec<Message> = messages[recent_start..].to_vec();

    // Build new messages with summary as first message
    let mut new_messages = vec![Message::User {
        content: format!(
            "## Continuing from Compacted Context\n\n{}\n\n---\n\n[The above is a summary of our previous conversation. Please continue with the task.]",
            summary
        ),
    }];
    new_messages.extend(recent_messages);

    let new_tokens = estimate_tokens(&new_messages);
    let ratio = new_tokens as f32 / original_tokens as f32;

    Ok(CompactedContext {
        summary,
        messages: new_messages,
        compaction_ratio: ratio,
        used_full_summarization: true,
    })
}

/// Check if an error message indicates a rate limit (429 / quota exceeded)
/// These should be retried, not compacted.
pub fn is_rate_limit_error(error_msg: &str) -> bool {
    let error_lower = error_msg.to_lowercase();

    (error_lower.contains("429") || error_lower.contains("too many requests"))
        || (error_lower.contains("quota") && error_lower.contains("exceeded"))
        || (error_lower.contains("rate") && error_lower.contains("limit"))
}

/// Extract retry delay from Gemini error message (e.g., "Please retry in 43.029546932s.")
/// Returns delay in seconds, or None if not found.
pub fn extract_retry_delay(error_msg: &str) -> Option<u64> {
    // Look for "retry in Xs" or "retryDelay": "Xs"
    if let Some(idx) = error_msg.find("retry in ") {
        let after = &error_msg[idx + 9..];
        if let Some(end) = after.find('s') {
            if let Ok(secs) = after[..end].trim().parse::<f64>() {
                return Some(secs.ceil() as u64);
            }
        }
    }
    if let Some(idx) = error_msg.find("\"retryDelay\": \"") {
        let after = &error_msg[idx + 15..];
        if let Some(end) = after.find('s') {
            if let Ok(secs) = after[..end].trim().parse::<f64>() {
                return Some(secs.ceil() as u64);
            }
        }
    }
    None
}

/// Check if an error message indicates context exhaustion (token limit exceeded)
/// Note: Rate limit errors (quota/429) are excluded - use is_rate_limit_error for those.
pub fn is_context_exhausted_error(error_msg: &str) -> bool {
    // Rate limit errors should NOT be treated as context exhaustion
    if is_rate_limit_error(error_msg) {
        return false;
    }

    let error_lower = error_msg.to_lowercase();

    // Gemini context errors (not quota-related)
    error_lower.contains("resource_exhausted")
        || error_lower.contains("resource exhausted")
        // OpenAI errors
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context length")
        // Anthropic errors
        || error_lower.contains("prompt is too long")
        // Generic patterns
        || (error_lower.contains("token") && error_lower.contains("limit"))
        || (error_lower.contains("token") && error_lower.contains("exceed"))
        || (error_lower.contains("context") && error_lower.contains("exceed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let messages = vec![
            Message::User {
                content: "Hello world".to_string(), // ~3 tokens
            },
            Message::Assistant {
                content: Some("Hi there!".to_string()), // ~2 tokens
                tool_calls: None,
            },
        ];

        let tokens = estimate_tokens(&messages);
        assert!(tokens > 0);
        assert!(tokens < 20); // Should be small
    }

    #[test]
    fn test_lightweight_compact() {
        let config = CompactionConfig {
            preserve_recent_messages: 2,
            tool_output_max_chars: 50,
            ..Default::default()
        };

        let messages = vec![
            Message::User {
                content: "Do something".to_string(),
            },
            Message::Tool {
                tool_call_id: "call_1".to_string(),
                content: "A".repeat(1000), // Long output
            },
            Message::User {
                content: "Recent message 1".to_string(),
            },
            Message::User {
                content: "Recent message 2".to_string(),
            },
        ];

        let compacted = lightweight_compact(&messages, &config);

        // Old tool output should be truncated
        if let Message::Tool { content, .. } = &compacted[1] {
            assert!(content.len() < 500);
            assert!(content.contains("truncated") || content.contains("chars"));
        } else {
            panic!("Expected Tool message");
        }

        // Recent messages should be preserved
        if let Message::User { content } = &compacted[3] {
            assert_eq!(content, "Recent message 2");
        }
    }

    #[test]
    fn test_is_rate_limit_error() {
        // 429 errors
        assert!(is_rate_limit_error("API request failed with status 429 Too Many Requests: {}"));
        assert!(is_rate_limit_error("Too Many Requests"));

        // Quota errors
        assert!(is_rate_limit_error("You exceeded your current quota, please check your plan"));
        assert!(is_rate_limit_error("RESOURCE_EXHAUSTED: quota exceeded"));
        assert!(is_rate_limit_error("Quota exceeded for metric: generativelanguage"));

        // Rate limit errors
        assert!(is_rate_limit_error("rate limit exceeded"));

        // Should not match non-rate-limit errors
        assert!(!is_rate_limit_error("Connection timeout"));
        assert!(!is_rate_limit_error("The input token count exceeds the maximum"));
        assert!(!is_rate_limit_error("Resource exhausted error"));
    }

    #[test]
    fn test_extract_retry_delay() {
        // Gemini format
        assert_eq!(
            extract_retry_delay("Please retry in 43.029546932s."),
            Some(44) // ceil of 43.03
        );
        assert_eq!(
            extract_retry_delay("Please retry in 10s."),
            Some(10)
        );

        // retryDelay JSON format
        assert_eq!(
            extract_retry_delay(r#""retryDelay": "43s""#),
            Some(43)
        );

        // No delay found
        assert_eq!(extract_retry_delay("Some other error"), None);
    }

    #[test]
    fn test_is_context_exhausted_error() {
        // Gemini context errors (not quota-related)
        assert!(is_context_exhausted_error("Resource exhausted error"));

        // Gemini 400 token count error
        assert!(is_context_exhausted_error(
            "The input token count exceeds the maximum number of tokens allowed (1048576)."
        ));

        // OpenAI errors
        assert!(is_context_exhausted_error(
            "This model's maximum context length is 8192 tokens"
        ));
        assert!(is_context_exhausted_error("context_length_exceeded"));

        // Rate limit errors should NOT match context exhaustion
        assert!(!is_context_exhausted_error("RESOURCE_EXHAUSTED: quota exceeded"));
        assert!(!is_context_exhausted_error("429 Too Many Requests: quota exceeded"));

        // Should not match random errors
        assert!(!is_context_exhausted_error("Connection timeout"));
        assert!(!is_context_exhausted_error("Invalid API key"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("longer string here", 10), "longer str...[truncated]");
    }

    #[test]
    fn test_format_conversation() {
        let messages = vec![
            Message::User {
                content: "Hello".to_string(),
            },
            Message::Assistant {
                content: Some("Hi!".to_string()),
                tool_calls: None,
            },
        ];

        let formatted = format_conversation_for_summary(&messages);
        assert!(formatted.contains("USER:"));
        assert!(formatted.contains("ASSISTANT:"));
        assert!(formatted.contains("Hello"));
        assert!(formatted.contains("Hi!"));
    }
}
