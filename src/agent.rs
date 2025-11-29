use crate::client::Client;
use crate::display;
use crate::display::{Spinner, ThinkingSpinner};
use crate::mcp::McpManager;
use crate::models::{FunctionSpec, Message, Tool};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};

// --- Built-in interpret_image tool ---
const INTERPRET_IMAGE_TOOL_NAME: &str = "interpret_image";

fn get_interpret_image_tool_spec() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionSpec {
            name: INTERPRET_IMAGE_TOOL_NAME.to_string(),
            description: "Analyzes an image file and returns a text description. The user's request will be used to guide the analysis.".to_string(),
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

async fn execute_interpret_image(
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
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
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

/// Run the agent loop until completion
pub async fn run_agent(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    mut mcp_manager: Option<&mut McpManager>,
    silent: bool,
    verbose: bool,
    conversation_history: &mut Vec<Message>,
    enable_image_tool: bool,
) -> Result<()> {
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

        let tools_option = if tools.is_empty() { None } else { Some(tools.as_slice()) };

        display::debug("Calling LLM...", verbose);

        // Start thinking spinner
        let thinking_spinner = if !silent {
            Some(ThinkingSpinner::start())
        } else {
            None
        };

        // Call the LLM
        let response = client
            .chat_completion(
                model,
                serde_json::to_value(&*conversation_history)?,
                tools_option.as_deref(),
                enable_image_tool,
            )
            .await;

        // Stop thinking spinner
        if let Some(spinner) = thinking_spinner {
            spinner.stop();
        }

        let response = response?;

        let choice = &response.choices[0];

        // Add assistant response to history
        let assistant_message = Message::Assistant {
            content: choice.message.content.clone(),
            tool_calls: choice.message.tool_calls.clone(),
        };
        conversation_history.push(assistant_message);

        // Display content if present (always show, even in silent mode)
        if let Some(content) = &choice.message.content {
            if !content.is_empty() {
                println!("{}", content);
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
                display::print_tool_call(tool_name, arguments);
            }

            // Parse arguments
            let args: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();

            // Start spinner for tool execution
            let spinner = if !silent {
                Some(Spinner::start(&format!("Running {}", tool_name)))
            } else {
                None
            };

            // Execute tool via MCP manager
            let result = if tool_name == INTERPRET_IMAGE_TOOL_NAME {
                execute_interpret_image(client, model, args).await
            } else if let Some(ref mut manager) = mcp_manager.as_deref_mut() {
                manager.execute_tool(tool_name, args).await
            } else {
                Ok("Error: No MCP manager available".to_string())
            }
            .unwrap_or_else(|e| format!("Error: {}", e));

            // Stop spinner
            if let Some(spinner) = spinner {
                spinner.stop().await;
            }

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

    Ok(())
}
