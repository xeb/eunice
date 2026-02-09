use crate::client::Client;
use crate::compact::{compact_context, is_context_exhausted_error, CompactionConfig};
use crate::display_sink::{DisplayEvent, DisplaySink};
use crate::models::{FunctionSpec, Message, Tool};
use crate::output_store::OutputStore;
use crate::tools::ToolRegistry;
use crate::usage::SessionUsage;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::watch;

// --- Built-in get_output tool ---
pub const GET_OUTPUT_TOOL_NAME: &str = "get_output";

/// Get the tool spec for the built-in get_output tool
pub fn get_get_output_tool_spec() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionSpec {
            name: GET_OUTPUT_TOOL_NAME.to_string(),
            description: "Retrieve a range of lines from a previous tool output by its ID. Tool outputs that are too large are automatically truncated, showing the first and last 10 lines. Use this tool to retrieve the middle sections or re-read specific ranges.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The output ID (e.g., 'out_001') shown in the truncated output header."
                    },
                    "start": {
                        "type": "integer",
                        "description": "Start line number (0-indexed). Defaults to 0."
                    },
                    "end": {
                        "type": "integer",
                        "description": "End line number (exclusive). Defaults to start + 100."
                    }
                },
                "required": ["id"]
            }),
        },
    }
}

/// Execute get_output tool to retrieve stored output ranges
pub fn execute_get_output(
    output_store: &OutputStore,
    args: serde_json::Value,
) -> Result<String> {
    let id = args["id"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing id parameter"))?;

    let start = args["start"]
        .as_u64()
        .map(|n| n as usize)
        .unwrap_or(0);

    let end = args["end"]
        .as_u64()
        .map(|n| Some(n as usize))
        .unwrap_or(None);

    output_store.get_range(id, start, end)
}

/// Result of running the agent - indicates if it completed or was cancelled
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub status: AgentStatus,
    pub usage: crate::usage::SessionUsage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Completed,
    Cancelled,
}

/// Run the agent loop until completion
#[allow(clippy::too_many_arguments)]
pub async fn run_agent(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    tool_registry: &ToolRegistry,
    display: Arc<dyn DisplaySink>,
    conversation_history: &mut Vec<Message>,
    compaction_config: Option<CompactionConfig>,
    output_store: Option<&mut OutputStore>,
) -> Result<AgentResult> {
    run_agent_cancellable(
        client,
        model,
        prompt,
        tool_output_limit,
        tool_registry,
        display,
        conversation_history,
        None,
        compaction_config,
        output_store,
    )
    .await
}

/// Run the agent loop with optional cancellation support
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_cancellable(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    tool_registry: &ToolRegistry,
    display: Arc<dyn DisplaySink>,
    conversation_history: &mut Vec<Message>,
    cancel_rx: Option<watch::Receiver<bool>>,
    compaction_config: Option<CompactionConfig>,
    mut output_store: Option<&mut OutputStore>,
) -> Result<AgentResult> {
    // Add user message to history
    conversation_history.push(Message::User {
        content: prompt.to_string(),
    });

    // Track if we've already tried compression this loop iteration
    let mut compaction_attempted = false;

    // Track token usage across API calls
    let mut session_usage = SessionUsage::new();

    loop {
        // Get available tools
        let mut tools = tool_registry.get_tools();

        // Add built-in get_output tool when output_store is enabled
        if output_store.is_some() {
            tools.push(get_get_output_tool_spec());
        }

        let tools_option = if tools.is_empty() { None } else { Some(tools.as_slice()) };

        // Track whether we used streaming (to skip duplicate Response display)
        let mut used_streaming = false;

        // Call the LLM - use streaming if available
        let response = if client.supports_streaming() {
            // Streaming mode - show thinking until first chunk arrives
            display.write_event(DisplayEvent::ThinkingStart);
            let display_clone = Arc::clone(&display);
            let mut streamed_any = false;

            let api_call = client.chat_completion_streaming(
                model,
                serde_json::to_value(&*conversation_history)?,
                tools_option.as_deref(),
                |chunk| {
                    if !streamed_any {
                        // First chunk - stop thinking indicator
                        display_clone.write_event(DisplayEvent::ThinkingStop);
                    }
                    streamed_any = true;
                    display_clone.write_event(DisplayEvent::StreamChunk {
                        content: chunk.to_string(),
                    });
                },
            );

            let result = if let Some(ref mut rx) = cancel_rx.clone() {
                tokio::select! {
                    result = api_call => result,
                    _ = rx.changed() => {
                        if streamed_any {
                            display.write_event(DisplayEvent::StreamEnd);
                        }
                        return Ok(AgentResult {
                            status: AgentStatus::Cancelled,
                            usage: session_usage,
                        });
                    }
                }
            } else {
                api_call.await
            };

            // End the stream if we streamed any content
            if streamed_any {
                display.write_event(DisplayEvent::StreamEnd);
                used_streaming = true;
            } else {
                // No content streamed - stop thinking indicator
                display.write_event(DisplayEvent::ThinkingStop);
            }

            result
        } else {
            // Non-streaming mode - show thinking indicator
            display.write_event(DisplayEvent::ThinkingStart);

            let api_call = client.chat_completion(
                model,
                serde_json::to_value(&*conversation_history)?,
                tools_option.as_deref(),
            );

            if let Some(ref mut rx) = cancel_rx.clone() {
                tokio::select! {
                    result = api_call => {
                        display.write_event(DisplayEvent::ThinkingStop);
                        result
                    }
                    _ = rx.changed() => {
                        display.write_event(DisplayEvent::ThinkingStop);
                        return Ok(AgentResult {
                            status: AgentStatus::Cancelled,
                            usage: session_usage,
                        });
                    }
                }
            } else {
                let result = api_call.await;
                display.write_event(DisplayEvent::ThinkingStop);
                result
            }
        };

        // Handle errors with potential context compression
        let response = match response {
            Ok(r) => {
                compaction_attempted = false; // Reset on success
                // Track usage if available
                if let Some(ref usage) = r.usage {
                    session_usage.add(usage);
                }
                r
            }
            Err(e) => {
                // Use {:#} to get full error chain from anyhow
                let error_msg = format!("{:#}", e);

                // Check if this is a context exhaustion error and we can compact
                if is_context_exhausted_error(&error_msg)
                    && !compaction_attempted
                    && compaction_config.is_some()
                {
                    let config = compaction_config.as_ref().unwrap();
                    if config.enabled {
                        // Attempt compaction
                        match compact_context(client, model, conversation_history, config).await {
                            Ok(compacted) => {
                                // Replace conversation history with compacted version
                                conversation_history.clear();
                                conversation_history.extend(compacted.messages);

                                compaction_attempted = true;
                                continue; // Retry with compacted context
                            }
                            Err(compact_err) => {
                                display.write_event(DisplayEvent::Error {
                                    message: format!("Compaction failed: {:#}", compact_err),
                                });
                                return Err(e); // Return original error
                            }
                        }
                    }
                }

                // Not a context error or compaction disabled/failed
                return Err(e);
            }
        };

        let choice = &response.choices[0];

        // Add assistant response to history
        let assistant_message = Message::Assistant {
            content: choice.message.content.clone(),
            tool_calls: choice.message.tool_calls.clone(),
        };
        conversation_history.push(assistant_message);

        // Display content if present (skip if we already streamed it)
        if !used_streaming {
            if let Some(content) = &choice.message.content {
                display.write_event(DisplayEvent::Response {
                    content: content.clone(),
                });
            }
        }

        // Check for tool calls
        let Some(tool_calls) = &choice.message.tool_calls else {
            break;
        };

        if tool_calls.is_empty() {
            break;
        }

        // Execute each tool call
        for tool_call in tool_calls {
            let tool_name = &tool_call.function.name;
            let arguments = &tool_call.function.arguments;

            // Display tool call with arguments
            display.write_event(DisplayEvent::ToolCall {
                name: tool_name.clone(),
                arguments: arguments.clone(),
            });

            // Parse arguments
            let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();

            // Execute tool
            let result = if tool_name == GET_OUTPUT_TOOL_NAME {
                // Handle get_output tool
                if let Some(ref store) = output_store {
                    execute_get_output(store, args)
                        .unwrap_or_else(|e| format!("Error: {}", e))
                } else {
                    "Error: Output store not available".to_string()
                }
            } else if tool_registry.has_tool(tool_name) {
                // Execute via ToolRegistry
                let raw_result = tool_registry.execute(tool_name, args).await
                    .unwrap_or_else(|e| format!("Error: {}", e));

                // Store output if store is enabled
                if let Some(ref mut store) = output_store {
                    match store.store(raw_result) {
                        Ok((_id, truncated)) => truncated,
                        Err(_) => "Error: Failed to store output".to_string(),
                    }
                } else {
                    raw_result
                }
            } else {
                format!("Error: Unknown tool '{}'", tool_name)
            };

            // Display result
            display.write_event(DisplayEvent::ToolResult {
                result: result.clone(),
                limit: tool_output_limit,
            });

            // Add tool result to history
            conversation_history.push(Message::Tool {
                tool_call_id: tool_call.id.clone(),
                content: result,
            });
        }
    }

    Ok(AgentResult {
        status: AgentStatus::Completed,
        usage: session_usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_output_tool_spec() {
        let spec = get_get_output_tool_spec();
        assert_eq!(spec.function.name, GET_OUTPUT_TOOL_NAME);
        assert_eq!(spec.tool_type, "function");
        assert!(spec.function.description.contains("Retrieve"));
        assert!(spec.function.description.contains("truncated"));

        // Check parameters
        let params = &spec.function.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["id"]["type"].as_str() == Some("string"));
        assert!(params["properties"]["start"]["type"].as_str() == Some("integer"));
        assert!(params["properties"]["end"]["type"].as_str() == Some("integer"));

        // Check required fields - only id is required
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("id")));
        assert!(!required.contains(&serde_json::json!("start")));
        assert!(!required.contains(&serde_json::json!("end")));
    }

    #[test]
    fn test_execute_get_output() {
        let mut store = OutputStore::new();

        // Store some content
        let lines: Vec<String> = (1..=200).map(|i| format!("line {}", i)).collect();
        let content = lines.join("\n");
        let (id, _) = store.store(content).unwrap();

        // Test retrieval
        let args = serde_json::json!({
            "id": id,
            "start": 50,
            "end": 60
        });

        let result = execute_get_output(&store, args).unwrap();
        assert!(result.contains("line 51")); // 0-indexed
        assert!(result.contains("line 60"));
    }
}
