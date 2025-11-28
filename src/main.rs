mod agent;
mod client;
mod config;
mod display;
mod interactive;
mod mcp;
mod models;
mod orchestrator;
mod provider;

use crate::client::Client;
use crate::config::{get_dmn_mcp_config, load_mcp_config, DMN_INSTRUCTIONS};
use crate::mcp::McpManager;
use crate::models::{McpConfig, Message};
use crate::orchestrator::AgentOrchestrator;
use crate::provider::{detect_provider, get_smart_default_model};
use anyhow::{anyhow, Result};
use clap::{CommandFactory, Parser};
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

    /// Run as a specific agent (uses 'root' by default if agents configured)
    #[arg(long)]
    agent: Option<String>,

    /// List configured agents
    #[arg(long)]
    list_agents: bool,

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

/// Print MCP server info for help output
fn print_mcp_servers_help(config: &McpConfig) {
    let mut servers: Vec<_> = config.mcp_servers.keys().collect();
    servers.sort();
    for name in servers {
        if let Some(server) = config.mcp_servers.get(name) {
            let args = server.args.join(" ");
            println!("    {}: {} {}", name, server.command, args);
        }
    }
}

/// Handle --help with MCP server info
fn handle_help_with_mcp() {
    let args: Vec<String> = std::env::args().collect();

    // Check if help is requested
    let has_help = args.iter().any(|a| a == "-h" || a == "--help");
    if !has_help {
        return;
    }

    // Print standard help
    Args::command().print_help().unwrap();
    println!("\n");

    // Check for --config
    let config_path = args.iter()
        .position(|a| a == "--config")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    if let Some(path) = config_path {
        // Load and display custom config
        match load_mcp_config(Path::new(path)) {
            Ok(config) => {
                println!("MCP Servers (from {}):", path);
                print_mcp_servers_help(&config);
            }
            Err(e) => {
                eprintln!("Error loading config '{}': {}", path, e);
            }
        }
    } else {
        // Show DMN servers
        println!("DMN Mode MCP Servers (--dmn):");
        let dmn_config = get_dmn_mcp_config();
        print_mcp_servers_help(&dmn_config);
    }

    std::process::exit(0);
}

/// Auto-discover prompt files in priority order
const PROMPT_FILES: &[&str] = &[
    "prompt.txt",
    "prompt.md",
    "instruction.txt",
    "instruction.md",
    "instructions.txt",
    "instructions.md",
];

/// Resolve prompt from arguments (may be a file path or string)
fn resolve_prompt(args: &Args) -> Result<Option<String>> {
    let some_prompt = args
        .prompt
        .clone()
        .or_else(|| args.prompt_positional.clone());

    if let Some(prompt) = some_prompt {
        let path = Path::new(&prompt);
        if path.exists() && path.is_file() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow!("Failed to read prompt file '{}': {}", prompt, e))?;
            return Ok(Some(content));
        }
        return Ok(Some(prompt));
    }

    // Auto-discover prompt files if no prompt specified
    for filename in PROMPT_FILES {
        let path = Path::new(filename);
        if path.exists() && path.is_file() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow!("Failed to read prompt file '{}': {}", filename, e))?;
            return Ok(Some(content));
        }
    }

    Ok(None)
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

    // Auto-discover eunice.toml or eunice.json in current directory (prefer TOML)
    let toml_config = Path::new("eunice.toml");
    let json_config = Path::new("eunice.json");

    if toml_config.exists() && json_config.exists() {
        eprintln!("Warning: Both eunice.toml and eunice.json exist. Using eunice.toml.");
    }

    if toml_config.exists() {
        return Ok(Some(load_mcp_config(toml_config)?));
    }

    if json_config.exists() {
        return Ok(Some(load_mcp_config(json_config)?));
    }

    Ok(None)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle --help with MCP server info before clap parses
    handle_help_with_mcp();

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

    // Get config early for --list-agents
    let mcp_config = determine_config(&args)?;

    // Handle --list-agents
    if args.list_agents {
        if let Some(ref config) = mcp_config {
            if config.agents.is_empty() {
                println!("No agents configured.");
            } else {
                println!("Configured agents:");
                let mut names: Vec<_> = config.agents.keys().collect();
                names.sort();
                for name in names {
                    if let Some(agent) = config.agents.get(name) {
                        let tools_str = if agent.tools.is_empty() {
                            "no tools".to_string()
                        } else {
                            agent.tools.join(", ")
                        };
                        let invoke_str = if agent.can_invoke.is_empty() {
                            "".to_string()
                        } else {
                            format!(" → can invoke: {}", agent.can_invoke.join(", "))
                        };
                        println!("  {}: [{}]{}", name, tools_str, invoke_str);
                    }
                }
            }
        } else {
            println!("No configuration loaded.");
        }
        return Ok(());
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

    // Initialize MCP manager (background startup for faster prompt display)
    let (mut mcp_manager, orchestrator) = if let Some(ref config) = mcp_config {
        let mut manager = McpManager::new();
        // Start servers in background - they'll be awaited when tools are called
        manager.start_servers_background(config, args.silent);

        // Create orchestrator if agents are configured
        let orch = if !config.agents.is_empty() {
            Some(AgentOrchestrator::new(config, None)?)
        } else {
            None
        };

        (Some(manager), orch)
    } else {
        (None, None)
    };

    // Determine agent to use
    let agent_name = if let Some(ref name) = args.agent {
        Some(name.clone())
    } else if orchestrator.as_ref().map_or(false, |o| o.has_agents()) {
        Some("root".to_string()) // Default to root agent
    } else {
        None
    };

    // Validate agent exists
    if let Some(ref name) = agent_name {
        if let Some(ref orch) = orchestrator {
            if orch.get_agent(name).is_none() {
                let available = orch.agent_names().join(", ");
                return Err(anyhow!("Unknown agent '{}'. Available: {}", name, available));
            }
        } else {
            return Err(anyhow!("--agent specified but no agents configured"));
        }
    }

    // Run appropriate mode
    if interactive {
        interactive::interactive_mode(
            &client,
            &provider_info.resolved_model,
            prompt.as_deref(),
            args.tool_output_limit,
            mcp_manager.as_mut(),
            orchestrator.as_ref(),
            agent_name.as_deref(),
            args.silent,
            args.verbose,
            args.dmn,
        )
        .await?;
    } else {
        // Single-shot mode
        let prompt = prompt.unwrap();

        // Show model/MCP info
        if !args.silent {
            display::print_model_info(&provider_info.resolved_model, &provider_info.provider);

            if let Some(ref mut manager) = mcp_manager {
                // Wait for servers to be ready before displaying info
                manager.await_all_servers().await;
                let server_info = manager.get_server_info();
                display::print_mcp_info(&server_info);
            }

            if args.dmn {
                display::print_dmn_mode();
            }

            if let Some(ref name) = agent_name {
                eprintln!("🤖 Multi-Agent Mode: starting as '{}'", name);
            }
        }

        // Run in multi-agent mode or single-agent mode
        if let (Some(ref orch), Some(ref name), Some(ref mut manager)) = (&orchestrator, &agent_name, &mut mcp_manager) {
            // Multi-agent mode
            orch.run_agent(
                &client,
                &provider_info.resolved_model,
                name,
                &prompt,
                None,
                manager,
                args.tool_output_limit,
                args.silent,
                args.verbose,
                0,
            ).await?;
        } else {
            // Single-agent mode (original behavior)
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
                args.dmn,
            )
            .await?;
        }
    }

    // Cleanup MCP servers
    if let Some(mut manager) = mcp_manager {
        manager.shutdown().await?;
    }

    Ok(())
}
