mod agent;
mod client;
mod config;
mod display;
mod interactive;
mod mcp;
mod models;
mod provider;

use crate::client::Client;
use crate::config::{get_dmn_mcp_config, load_mcp_config, DMN_INSTRUCTIONS};
use crate::mcp::McpManager;
use crate::models::{McpConfig, Message};
use crate::provider::{detect_provider, get_smart_default_model};
use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::Path;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "eunice", about = "Agentic CLI runner with OpenAI, Gemini, Claude, and Ollama support", version = VERSION)]
struct Args {
    /// AI model to use
    #[arg(long)]
    model: Option<String>,

    /// Prompt as file path or string
    #[arg(long)]
    prompt: Option<String>,

    /// Limit tool output display (0=unlimited)
    #[arg(long, default_value = "50")]
    tool_output_limit: usize,

    /// Show all available models
    #[arg(long)]
    list_models: bool,

    /// Path to MCP configuration JSON
    #[arg(long)]
    config: Option<String>,

    /// Disable MCP even if eunice.json exists
    #[arg(long)]
    no_mcp: bool,

    /// Enable DMN (Default Mode Network) with auto-loaded MCP tools
    #[arg(long = "default-mode-network", visible_alias = "dmn")]
    dmn: bool,

    /// Interactive mode for multi-turn conversations
    #[arg(long, short = 'i')]
    interact: bool,

    /// Suppress all output except AI responses
    #[arg(long)]
    silent: bool,

    /// Enable verbose debug output
    #[arg(long)]
    verbose: bool,

    /// Output JSON-RPC events to stdout
    #[arg(long)]
    events: bool,

    /// Positional prompt argument
    #[arg()]
    prompt_positional: Option<String>,
}

/// Resolve prompt from arguments (may be a file path or string)
fn resolve_prompt(args: &Args) -> Result<Option<String>> {
    let prompt_arg = args
        .prompt
        .clone()
        .or_else(|| args.prompt_positional.clone());

    let Some(prompt) = prompt_arg else {
        return Ok(None);
    };

    // Quick heuristics to determine if it's definitely a string
    if prompt.len() > 255
        || prompt.contains('\n')
        || prompt.contains('?')
        || prompt.matches(' ').count() > 5
    {
        return Ok(Some(prompt));
    }

    // Check if it looks like a file path and exists
    let path = Path::new(&prompt);
    if path.exists() && path.is_file() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read prompt file '{}': {}", prompt, e))?;
        return Ok(Some(content));
    }

    // If it looks like a file path but doesn't exist, error
    if prompt.contains('/') || prompt.contains('\\') || prompt.ends_with(".txt") || prompt.ends_with(".md") {
        if !path.exists() {
            return Err(anyhow!("Prompt file '{}' does not exist", prompt));
        }
    }

    Ok(Some(prompt))
}

/// Determine the MCP configuration to use
fn determine_config(args: &Args) -> Result<Option<McpConfig>> {
    // --no-mcp disables MCP
    if args.no_mcp {
        return Ok(None);
    }

    // --dmn uses embedded config
    if args.dmn {
        if args.config.is_some() {
            return Err(anyhow!("--dmn cannot be used with --config"));
        }
        return Ok(Some(get_dmn_mcp_config()));
    }

    // --config specified
    if let Some(config_path) = &args.config {
        // Empty config = no MCP
        if config_path.is_empty() {
            return Ok(None);
        }

        let path = Path::new(config_path);
        return Ok(Some(load_mcp_config(path)?));
    }

    // Auto-discover eunice.json in current directory
    let default_config = Path::new("eunice.json");
    if default_config.exists() {
        return Ok(Some(load_mcp_config(default_config)?));
    }

    Ok(None)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --list-models
    if args.list_models {
        display::print_model_list();
        return Ok(());
    }

    // Validate conflicting arguments
    if args.no_mcp && args.config.is_some() {
        return Err(anyhow!("--no-mcp and --config cannot be used together"));
    }

    if args.dmn && args.no_mcp {
        return Err(anyhow!("--dmn requires MCP tools and cannot be used with --no-mcp"));
    }

    // Resolve prompt
    let prompt = resolve_prompt(&args)?;

    // Determine if we need interactive mode
    let interactive = args.interact || prompt.is_none();

    // Select model
    let model = match &args.model {
        Some(m) => m.clone(),
        None => get_smart_default_model()?,
    };

    // Detect provider and create client
    let provider_info = detect_provider(&model)?;
    let client = Client::new(&provider_info)?;

    // Initialize MCP manager
    let mcp_config = determine_config(&args)?;
    let mut mcp_manager = if let Some(config) = mcp_config {
        let mut manager = McpManager::new();
        manager.load_config(&config, args.silent).await?;
        Some(manager)
    } else {
        None
    };

    // Run appropriate mode
    if interactive {
        interactive::interactive_mode(
            &client,
            &provider_info.resolved_model,
            prompt.as_deref(),
            args.tool_output_limit,
            mcp_manager.as_mut(),
            args.silent,
            args.verbose,
            args.events,
            args.dmn,
        )
        .await?;
    } else {
        // Single-shot mode
        let prompt = prompt.unwrap();

        // Show model/MCP info
        if !args.silent {
            display::print_model_info(&provider_info.resolved_model, &provider_info.provider);

            if let Some(ref manager) = mcp_manager {
                let server_info = manager.get_server_info();
                display::print_mcp_info(&server_info);
            }

            if args.dmn {
                display::print_dmn_mode();
            }
        }

        // Inject DMN instructions if needed
        let final_prompt = if args.dmn {
            format!(
                "{}\n\n---\n\n# USER REQUEST\n\n{}\n\n---\n\nYou are now in DMN (Default Mode Network) autonomous batch mode. Execute the user request above completely using your available MCP tools. Do not stop for confirmation.",
                DMN_INSTRUCTIONS, prompt
            )
        } else {
            prompt
        };

        let mut conversation_history: Vec<Message> = Vec::new();

        agent::run_agent(
            &client,
            &provider_info.resolved_model,
            &final_prompt,
            args.tool_output_limit,
            mcp_manager.as_mut(),
            args.silent,
            args.verbose,
            &mut conversation_history,
            false,
            args.events,
            args.dmn,
        )
        .await?;
    }

    // Cleanup MCP servers
    if let Some(mut manager) = mcp_manager {
        manager.shutdown().await?;
    }

    Ok(())
}
