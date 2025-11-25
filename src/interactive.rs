use crate::agent::run_agent;
use crate::client::Client;
use crate::config::DMN_INSTRUCTIONS;
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
    dmn: bool,
) -> Result<()> {
    let mut conversation_history: Vec<Message> = Vec::new();
    let mut dmn_injected = false;

    // Show model/MCP info once at startup
    if !silent {
        display::print_model_info(model, client.provider());

        if let Some(ref manager) = mcp_manager {
            let server_info = manager.get_server_info();
            display::print_mcp_info(&server_info);
        }

        if dmn {
            display::print_dmn_mode();
        }
    }

    // Process initial prompt if provided
    if let Some(prompt) = initial_prompt {
        let final_prompt = inject_dmn_instructions_if_needed(prompt, &mut dmn_injected, dmn);
        run_agent(
            client,
            model,
            &final_prompt,
            tool_output_limit,
            mcp_manager.as_deref_mut(),
            silent,
            verbose,
            &mut conversation_history,
            dmn,
        )
        .await?;
    }

    // Interactive loop
    loop {
        print!("\n> ");
        io::stdout().flush()?;

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;
        if bytes_read == 0 {
            break; // EOF (Ctrl+D)
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        let final_prompt = inject_dmn_instructions_if_needed(input, &mut dmn_injected, dmn);

        if let Err(e) = run_agent(
            client,
            model,
            &final_prompt,
            tool_output_limit,
            mcp_manager.as_deref_mut(),
            silent,
            verbose,
            &mut conversation_history,
            dmn,
        )
        .await
        {
            display::print_error(&format!("Agent error: {}", e));
        }
    }

    Ok(())
}

/// Inject DMN instructions if in DMN mode and not already injected
fn inject_dmn_instructions_if_needed(
    prompt: &str,
    dmn_injected: &mut bool,
    dmn: bool,
) -> String {
    if dmn && !*dmn_injected {
        *dmn_injected = true;
        format!(
            "{}\n\n---\n\n# USER REQUEST\n\n{}\n\n---\n\nYou are now in DMN (Default Mode Network) autonomous batch mode. Execute the user request above completely using your available MCP tools. Do not stop for confirmation.",
            DMN_INSTRUCTIONS, prompt
        )
    } else {
        prompt.to_string()
    }
}
