use crate::agent::run_agent;
use crate::client::Client;
use crate::config::DMN_INSTRUCTIONS;
use crate::display;
use crate::mcp::McpManager;
use crate::models::Message;
use crate::orchestrator::AgentOrchestrator;
use anyhow::Result;
use std::io::{self, Write};

/// Run interactive mode for multi-turn conversations
pub async fn interactive_mode(
    client: &Client,
    model: &str,
    initial_prompt: Option<&str>,
    tool_output_limit: usize,
    mut mcp_manager: Option<&mut McpManager>,
    orchestrator: Option<&AgentOrchestrator>,
    agent_name: Option<&str>,
    silent: bool,
    verbose: bool,
    dmn: bool,
) -> Result<()> {
    let mut conversation_history: Vec<Message> = Vec::new();
    let mut dmn_injected = false;

    // Wait for MCP servers to be ready before showing prompt
    if let Some(ref mut manager) = mcp_manager {
        manager.await_all_servers().await;
    }

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

        if let Some(name) = agent_name {
            eprintln!("🤖 Multi-Agent Mode: starting as '{}'", name);
        }
    }

    // Process initial prompt if provided
    if let Some(prompt) = initial_prompt {
        run_prompt(
            client,
            model,
            prompt,
            tool_output_limit,
            &mut mcp_manager,
            orchestrator,
            agent_name,
            silent,
            verbose,
            dmn,
            &mut dmn_injected,
            &mut conversation_history,
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

        if let Err(e) = run_prompt(
            client,
            model,
            input,
            tool_output_limit,
            &mut mcp_manager,
            orchestrator,
            agent_name,
            silent,
            verbose,
            dmn,
            &mut dmn_injected,
            &mut conversation_history,
        )
        .await
        {
            display::print_error(&format!("Agent error: {}", e));
        }
    }

    Ok(())
}

/// Run a single prompt - either through orchestrator (multi-agent) or directly
async fn run_prompt(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    mcp_manager: &mut Option<&mut McpManager>,
    orchestrator: Option<&AgentOrchestrator>,
    agent_name: Option<&str>,
    silent: bool,
    verbose: bool,
    dmn: bool,
    dmn_injected: &mut bool,
    conversation_history: &mut Vec<Message>,
) -> Result<()> {
    // Use orchestrator if available (multi-agent mode)
    let use_multi_agent = orchestrator.is_some() && agent_name.is_some() && mcp_manager.is_some();

    if use_multi_agent {
        let orch = orchestrator.unwrap();
        let name = agent_name.unwrap();
        let manager = mcp_manager.as_mut().unwrap();
        orch.run_agent(
            client,
            model,
            name,
            prompt,
            None,
            manager,
            tool_output_limit,
            silent,
            verbose,
            0,
        )
        .await?;
    } else {
        // Single-agent mode (original behavior)
        let final_prompt = inject_dmn_instructions_if_needed(prompt, dmn_injected, dmn);
        run_agent(
            client,
            model,
            &final_prompt,
            tool_output_limit,
            mcp_manager.as_deref_mut(),
            silent,
            verbose,
            conversation_history,
            dmn,
        )
        .await?;
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
