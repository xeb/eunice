use crate::agent::run_agent;
use crate::client::Client;
use crate::config::SYSADMIN_INSTRUCTIONS;
use crate::display;
use crate::mcp::McpManager;
use crate::models::Message;
use anyhow::Result;
use std::io::{self, Write};

/// Run interactive mode for multi-turn conversations
pub async fn interactive_mode(
    client: &Client,
    model: &str,
    initial_prompt: Option<&str>,
    tool_output_limit: usize,
    mut mcp_manager: Option<&mut McpManager>,
    silent: bool,
    verbose: bool,
    events_mode: bool,
    sysadmin: bool,
) -> Result<()> {
    let mut conversation_history: Vec<Message> = Vec::new();
    let mut sysadmin_injected = false;

    // Show model/MCP info once at startup
    if !silent {
        display::print_model_info(model, client.provider());

        if let Some(ref manager) = mcp_manager {
            let server_info = manager.get_server_info();
            display::print_mcp_info(&server_info);
        }

        if sysadmin {
            display::print_sysadmin_mode();
        }
    }

    // Process initial prompt if provided
    if let Some(prompt) = initial_prompt {
        let prompt_with_instructions = if sysadmin {
            sysadmin_injected = true;
            format!(
                "{}\n\n---\n\n# USER REQUEST\n\n{}\n\n---\n\nYou are now in sysadmin mode. Execute the user request above using your available MCP tools.",
                SYSADMIN_INSTRUCTIONS, prompt
            )
        } else {
            prompt.to_string()
        };

        run_agent(
            client,
            model,
            &prompt_with_instructions,
            tool_output_limit,
            mcp_manager.as_deref_mut().map(|m| m as &mut McpManager),
            silent,
            verbose,
            &mut conversation_history,
            true,
            events_mode,
        )
        .await?;
    }

    // Interactive loop
    loop {
        print!("\n> ");
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                display::print_error(&format!("Failed to read input: {}", e));
                continue;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        // Inject sysadmin instructions on first interactive input
        let prompt = if sysadmin && !sysadmin_injected {
            sysadmin_injected = true;
            format!(
                "{}\n\n---\n\n# USER REQUEST\n\n{}\n\n---\n\nYou are now in sysadmin mode. Execute the user request above using your available MCP tools.",
                SYSADMIN_INSTRUCTIONS, input
            )
        } else {
            input.to_string()
        };

        if let Err(e) = run_agent(
            client,
            model,
            &prompt,
            tool_output_limit,
            mcp_manager.as_deref_mut().map(|m| m as &mut McpManager),
            silent,
            verbose,
            &mut conversation_history,
            true,
            events_mode,
        )
        .await
        {
            display::print_error(&format!("Agent error: {}", e));
        }
    }

    Ok(())
}
