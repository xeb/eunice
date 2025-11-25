use crate::client::Client;
use crate::display;
use crate::display::Spinner;
use crate::mcp::McpManager;
use crate::models::Message;
use anyhow::Result;

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
    dmn_mode: bool,
) -> Result<()> {
    // Add user message to history
    conversation_history.push(Message::User {
        content: prompt.to_string(),
    });

    display::debug(&format!("Sending prompt: {}", prompt), verbose);

    loop {
        // Get available tools
        let tools = mcp_manager
            .as_ref()
            .map(|m| m.get_tools())
            .filter(|t| !t.is_empty());

        display::debug("Calling LLM...", verbose);

        // Call the LLM
        let response = client
            .chat_completion(model, conversation_history, tools.as_deref(), dmn_mode)
            .await?;

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
            let result = if let Some(ref mut manager) = mcp_manager.as_deref_mut() {
                match manager.execute_tool(tool_name, args).await {
                    Ok(result) => result,
                    Err(e) => format!("Error: {}", e),
                }
            } else {
                "Error: No MCP manager available".to_string()
            };

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
