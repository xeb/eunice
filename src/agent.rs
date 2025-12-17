use crate::client::Client;
use crate::compact::{compact_context, is_context_exhausted_error, CompactionConfig};
use crate::display;
use crate::display::ThinkingSpinner;
use crate::mcp::McpManager;
use crate::models::{FunctionSpec, Message, Tool};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tokio::sync::watch;

// --- Built-in interpret_image tool ---
pub const INTERPRET_IMAGE_TOOL_NAME: &str = "interpret_image";

// --- Built-in search_query tool ---
pub const SEARCH_QUERY_TOOL_NAME: &str = "search_query";

/// Get the tool spec for the built-in interpret_image tool
pub fn get_interpret_image_tool_spec() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionSpec {
            name: INTERPRET_IMAGE_TOOL_NAME.to_string(),
            description: "Analyzes an image or PDF file and returns a text description. Supports PNG, JPEG, GIF, WebP images and PDF documents. The user's request will be used to guide the analysis.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the image file to analyze."
                    },
                    "prompt": {
                        "type": "string",
                        "description": "A specific prompt to guide the image analysis."
                    }
                },
                "required": ["file_path", "prompt"]
            }),
        },
    }
}

pub async fn execute_interpret_image(
    client: &Client,
    model: &str,
    args: serde_json::Value,
) -> Result<String> {
    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path"))?;
    let prompt = args["prompt"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing prompt"))?;

    // Read the image file
    let image_bytes = std::fs::read(file_path)
        .with_context(|| format!("Failed to read image file: {}", file_path))?;

    // Guess MIME type from extension
    let mime_type = match std::path::Path::new(file_path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream", // Default
    };

    // Base64 encode the image
    let image_base64 = STANDARD.encode(&image_bytes);

    // Call the client's multimodal chat completion method
    let response = client
        .chat_completion_with_image(model, prompt, &image_base64, mime_type, true)
        .await?;

    // Extract text from the response
    let content = response
        .choices
        .get(0)
        .and_then(|c| c.message.content.as_ref())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "No text content received from model.".to_string());

    Ok(content)
}

/// Get the tool spec for the built-in search_query tool
pub fn get_search_query_tool_spec() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionSpec {
            name: SEARCH_QUERY_TOOL_NAME.to_string(),
            description: "Search the web using Gemini models with Google Search grounding. Use 'flash' for quick knowledge queries (fast, cheap), use 'pro' for medium complexity queries requiring deeper analysis, and use 'pro_preview' for the hardest queries requiring maximum reasoning capability.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to send to the model with Google Search grounding."
                    },
                    "model": {
                        "type": "string",
                        "enum": ["flash", "pro", "pro_preview"],
                        "description": "The Gemini model to use. 'flash' (gemini-2.5-flash) for quick queries, 'pro' (gemini-2.5-pro) for medium complexity, 'pro_preview' (gemini-3-pro-preview) for hardest queries."
                    }
                },
                "required": ["query", "model"]
            }),
        },
    }
}

/// Execute search_query tool using Gemini API with Google Search grounding
pub async fn execute_search_query(
    args: serde_json::Value,
) -> Result<String> {
    let query = args["query"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing query"))?;
    let model_choice = args["model"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing model"))?;

    // Map model choice to actual model name
    let model = match model_choice {
        "flash" => "gemini-2.5-flash",
        "pro" => "gemini-2.5-pro",
        "pro_preview" => "gemini-3-pro-preview",
        _ => return Err(anyhow!("Invalid model choice: {}. Must be 'flash', 'pro', or 'pro_preview'", model_choice)),
    };

    // Get API key from environment
    let api_key = std::env::var("GEMINI_API_KEY")
        .context("GEMINI_API_KEY environment variable not set")?;

    // Build the request
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let request_body = serde_json::json!({
        "contents": [
            {
                "parts": [
                    {"text": query}
                ]
            }
        ],
        "tools": [
            {
                "google_search": {}
            }
        ]
    });

    // Make the HTTP request
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("x-goog-api-key", &api_key)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to send search request to Gemini")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini search request failed: {}", error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse Gemini search response")?;

    // Extract the text response
    let text = response_json["candidates"]
        .get(0)
        .and_then(|c| c["content"]["parts"].as_array())
        .and_then(|parts| {
            parts.iter()
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .first()
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "No text response from search".to_string());

    // Extract grounding metadata if available
    let mut result = text;

    if let Some(grounding) = response_json["candidates"]
        .get(0)
        .and_then(|c| c.get("groundingMetadata"))
    {
        if let Some(sources) = grounding.get("groundingChunks").and_then(|c| c.as_array()) {
            let source_urls: Vec<String> = sources
                .iter()
                .filter_map(|s| {
                    let web = s.get("web")?;
                    let uri = web.get("uri")?.as_str()?;
                    let title = web.get("title").and_then(|t| t.as_str()).unwrap_or("Source");
                    Some(format!("- [{}]({})", title, uri))
                })
                .collect();

            if !source_urls.is_empty() {
                result = format!("{}\n\n**Sources:**\n{}", result, source_urls.join("\n"));
            }
        }
    }

    Ok(result)
}

/// Result of running the agent - indicates if it completed or was cancelled
#[derive(Debug, Clone, PartialEq)]
pub enum AgentResult {
    Completed,
    Cancelled,
}

/// Run the agent loop until completion
pub async fn run_agent(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    mcp_manager: Option<&mut McpManager>,
    silent: bool,
    verbose: bool,
    conversation_history: &mut Vec<Message>,
    enable_image_tool: bool,
    enable_search_tool: bool,
    compaction_config: Option<CompactionConfig>,
) -> Result<AgentResult> {
    run_agent_cancellable(
        client,
        model,
        prompt,
        tool_output_limit,
        mcp_manager,
        silent,
        verbose,
        conversation_history,
        enable_image_tool,
        enable_search_tool,
        None,
        compaction_config,
    )
    .await
}

/// Run the agent loop with optional cancellation support
pub async fn run_agent_cancellable(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    mut mcp_manager: Option<&mut McpManager>,
    silent: bool,
    verbose: bool,
    conversation_history: &mut Vec<Message>,
    enable_image_tool: bool,
    enable_search_tool: bool,
    cancel_rx: Option<watch::Receiver<bool>>,
    compaction_config: Option<CompactionConfig>,
) -> Result<AgentResult> {
    // Build the prompt, including any failed server info
    let final_prompt = if let Some(ref manager) = mcp_manager {
        let failed = manager.get_failed_servers();
        if failed.is_empty() {
            prompt.to_string()
        } else {
            let errors: Vec<String> = failed
                .iter()
                .map(|(name, err)| format!("- {}: {}", name, err))
                .collect();
            format!(
                "{}\n\n[SYSTEM NOTE: The following MCP servers failed to connect. You cannot use tools from these servers:\n{}]",
                prompt,
                errors.join("\n")
            )
        }
    } else {
        prompt.to_string()
    };

    // Add user message to history
    conversation_history.push(Message::User {
        content: final_prompt.clone(),
    });

    display::debug(&format!("Sending prompt: {}", final_prompt), verbose);

    // Track if we've already tried compression this loop iteration
    let mut compaction_attempted = false;

    loop {
        // Get available tools
        let mut tools = mcp_manager
            .as_ref()
            .map(|m| m.get_tools())
            .unwrap_or_default()
            .into_iter()
            .filter(|t| !t.function.name.is_empty())
            .collect::<Vec<_>>();

        // Add built-in interpret_image tool when enabled (via --dmn or --images)
        if enable_image_tool {
            tools.push(get_interpret_image_tool_spec());
        }

        // Add built-in search_query tool when enabled (via --dmn or --search)
        if enable_search_tool {
            tools.push(get_search_query_tool_spec());
        }

        let tools_option = if tools.is_empty() { None } else { Some(tools.as_slice()) };

        display::debug("Calling LLM...", verbose);

        // Start thinking spinner
        let thinking_spinner = if !silent {
            Some(ThinkingSpinner::start())
        } else {
            None
        };

        // Call the LLM with optional cancellation support
        let response = {
            let api_call = client.chat_completion(
                model,
                serde_json::to_value(&*conversation_history)?,
                tools_option.as_deref(),
                enable_image_tool,
            );

            if let Some(ref mut rx) = cancel_rx.clone() {
                tokio::select! {
                    result = api_call => {
                        // Stop thinking spinner
                        if let Some(spinner) = thinking_spinner {
                            spinner.stop();
                        }
                        result
                    }
                    _ = rx.changed() => {
                        // Stop thinking spinner and return cancelled
                        if let Some(spinner) = thinking_spinner {
                            spinner.stop();
                        }
                        return Ok(AgentResult::Cancelled);
                    }
                }
            } else {
                let result = api_call.await;
                // Stop thinking spinner
                if let Some(spinner) = thinking_spinner {
                    spinner.stop();
                }
                result
            }
        };

        // Handle errors with potential context compression
        let response = match response {
            Ok(r) => {
                compaction_attempted = false; // Reset on success
                r
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check if this is a context exhaustion error and we can compact
                if is_context_exhausted_error(&error_msg)
                    && !compaction_attempted
                    && compaction_config.is_some()
                {
                    let config = compaction_config.as_ref().unwrap();
                    if config.enabled {
                        if !silent {
                            eprintln!(
                                "⚠️  Context exhausted. Compacting conversation history..."
                            );
                        }

                        // Attempt compaction
                        match compact_context(client, model, conversation_history, config).await {
                            Ok(compacted) => {
                                if !silent {
                                    let method = if compacted.used_full_summarization {
                                        "full summarization"
                                    } else {
                                        "lightweight compaction"
                                    };
                                    eprintln!(
                                        "✓ Compacted to {:.0}% of original size using {}",
                                        compacted.compaction_ratio * 100.0,
                                        method
                                    );
                                }

                                // Replace conversation history with compacted version
                                conversation_history.clear();
                                conversation_history.extend(compacted.messages);

                                compaction_attempted = true;
                                continue; // Retry with compacted context
                            }
                            Err(compact_err) => {
                                if !silent {
                                    eprintln!("✗ Compaction failed: {}", compact_err);
                                }
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

        // Display content if present (always show, even in silent mode)
        if let Some(content) = &choice.message.content {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                println!("{}", trimmed);
            }
        }

        // Check for tool calls
        let Some(tool_calls) = &choice.message.tool_calls else {
            display::debug("No tool calls, agent loop complete", verbose);
            break;
        };

        if tool_calls.is_empty() {
            display::debug("Empty tool calls, agent loop complete", verbose);
            break;
        }

        display::debug(&format!("Processing {} tool call(s)", tool_calls.len()), verbose);

        // Execute each tool call
        for tool_call in tool_calls {
            let tool_name = &tool_call.function.name;
            let arguments = &tool_call.function.arguments;

            // Display tool call
            if !silent {
                display::print_tool_call(tool_name);
            }

            // Parse arguments
            let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();

            // Execute tool via MCP manager or built-in handlers
            let result = if tool_name == INTERPRET_IMAGE_TOOL_NAME {
                execute_interpret_image(client, model, args).await
            } else if tool_name == SEARCH_QUERY_TOOL_NAME {
                execute_search_query(args).await
            } else if let Some(ref mut manager) = mcp_manager.as_deref_mut() {
                manager.execute_tool(tool_name, args).await
            } else {
                Ok("Error: No MCP manager available".to_string())
            }
            .unwrap_or_else(|e| format!("Error: {}", e));

            // Display result
            if !silent {
                display::print_tool_result(&result, tool_output_limit);
            }

            // Add tool result to history
            conversation_history.push(Message::Tool {
                tool_call_id: tool_call.id.clone(),
                content: result,
            });
        }
    }

    Ok(AgentResult::Completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpret_image_tool_spec() {
        let spec = get_interpret_image_tool_spec();
        assert_eq!(spec.function.name, INTERPRET_IMAGE_TOOL_NAME);
        assert_eq!(spec.tool_type, "function");
        assert!(spec.function.description.contains("image"));

        // Check parameters
        let params = &spec.function.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["file_path"]["type"].as_str() == Some("string"));
        assert!(params["properties"]["prompt"]["type"].as_str() == Some("string"));

        // Check required fields
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("file_path")));
        assert!(required.contains(&serde_json::json!("prompt")));
    }

    #[test]
    fn test_search_query_tool_spec() {
        let spec = get_search_query_tool_spec();
        assert_eq!(spec.function.name, SEARCH_QUERY_TOOL_NAME);
        assert_eq!(spec.tool_type, "function");
        assert!(spec.function.description.contains("Search"));
        assert!(spec.function.description.contains("Google Search"));

        // Check parameters
        let params = &spec.function.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["query"]["type"].as_str() == Some("string"));
        assert!(params["properties"]["model"]["type"].as_str() == Some("string"));

        // Check enum values for model
        let model_enum = params["properties"]["model"]["enum"].as_array().unwrap();
        assert!(model_enum.contains(&serde_json::json!("flash")));
        assert!(model_enum.contains(&serde_json::json!("pro")));
        assert!(model_enum.contains(&serde_json::json!("pro_preview")));

        // Check required fields
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("query")));
        assert!(required.contains(&serde_json::json!("model")));
    }

    #[test]
    fn test_search_query_tool_description_contains_guidance() {
        let spec = get_search_query_tool_spec();
        let desc = &spec.function.description;

        // Should contain guidance for each model tier
        assert!(desc.contains("flash"));
        assert!(desc.contains("pro"));
        assert!(desc.contains("pro_preview"));
        assert!(desc.contains("quick"));
        assert!(desc.contains("medium complexity"));
        assert!(desc.contains("hardest"));
    }
}
