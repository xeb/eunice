mod agent;
mod client;
mod compact;
mod config;
mod display;
mod interactive;
mod mcp;
mod models;
mod orchestrator;
mod provider;

use crate::client::Client;
use crate::config::{get_dmn_mcp_config, load_mcp_config, DMN_INSTRUCTIONS, LLMS_TXT, LLMS_FULL_TXT};
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
    // === Main Options ===
    /// AI model to use
    #[arg(long, help_heading = "Main Options")]
    model: Option<String>,

    /// Prompt as file path or string
    #[arg(long, help_heading = "Main Options")]
    prompt: Option<String>,

    /// Path to MCP configuration (TOML or JSON)
    #[arg(long, help_heading = "Main Options")]
    config: Option<String>,

    /// Positional prompt argument
    #[arg(help_heading = "Main Options")]
    prompt_positional: Option<String>,

    // === Discovery ===
    /// List available AI models
    #[arg(long, help_heading = "Discovery")]
    list_models: bool,

    /// List configured agents
    #[arg(long, help_heading = "Discovery")]
    list_agents: bool,

    /// List discovered MCP tools with sanitized names
    #[arg(long, help_heading = "Discovery")]
    list_tools: bool,

    /// List configured MCP servers
    #[arg(long, help_heading = "Discovery")]
    list_mcp_servers: bool,

    // === Modes ===
    /// Interactive mode for multi-turn conversations
    #[arg(long, short = 'i', help_heading = "Modes")]
    interact: bool,

    /// Enable DMN (Default Mode Network) with auto-loaded MCP tools
    #[arg(long = "default-mode-network", visible_alias = "dmn", help_heading = "Modes")]
    dmn: bool,

    /// Run as a specific agent (uses 'root' by default if agents configured)
    #[arg(long, help_heading = "Modes")]
    agent: Option<String>,

    /// Enable built-in image interpretation tool
    #[arg(long, help_heading = "Modes")]
    images: bool,

    /// Enable built-in web search tool (uses Gemini with Google Search)
    #[arg(long, help_heading = "Modes")]
    search: bool,

    // === Output ===
    /// Suppress all output except AI responses
    #[arg(long, help_heading = "Output")]
    silent: bool,

    /// Enable verbose debug output
    #[arg(long, help_heading = "Output")]
    verbose: bool,

    /// Output JSON-RPC events to stdout
    #[arg(long, help_heading = "Output")]
    events: bool,

    /// Limit tool output display (0=unlimited)
    #[arg(long, default_value = "50", help_heading = "Output")]
    tool_output_limit: usize,

    /// Output llms.txt (LLM context index)
    #[arg(long = "llms-txt", help_heading = "Output")]
    llms_txt: bool,

    /// Output llms-full.txt (full LLM context)
    #[arg(long = "llms-full-txt", help_heading = "Output")]
    llms_full_txt: bool,

    // === Advanced ===
    /// Disable MCP even if eunice.json exists
    #[arg(long, help_heading = "Advanced")]
    no_mcp: bool,
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

    // Handle --llms-txt
    if args.llms_txt {
        print!("{}", LLMS_TXT);
        return Ok(());
    }

    // Handle --llms-full-txt
    if args.llms_full_txt {
        print!("{}", LLMS_FULL_TXT);
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

    // Handle --list-mcp-servers
    if args.list_mcp_servers {
        let enable_image_tool = args.dmn || args.images;
        let enable_search_tool = args.dmn || args.search;
        let mut has_output = false;

        // Show built-in tools section
        if enable_image_tool || enable_search_tool {
            println!("Built-in tools:\n");
            if enable_image_tool {
                println!("  interpret_image");
                println!("    Analyzes images (PNG, JPEG, GIF, WebP) and PDF documents");
            }
            if enable_search_tool {
                println!("  search_query");
                println!("    Web search using Gemini models with Google Search");
            }
            println!();
            has_output = true;
        }

        // Show MCP servers
        if let Some(ref config) = mcp_config {
            if !config.mcp_servers.is_empty() {
                println!("MCP servers ({}):\n", config.mcp_servers.len());
                let mut names: Vec<_> = config.mcp_servers.keys().collect();
                names.sort();
                for name in names {
                    if let Some(server) = config.mcp_servers.get(name) {
                        if let Some(ref url) = server.url {
                            // HTTP transport
                            println!("  {} [http]", name);
                            println!("    url: {}", url);
                        } else {
                            // stdio transport
                            let args_str = if server.args.is_empty() {
                                String::new()
                            } else {
                                format!(" {}", server.args.join(" "))
                            };
                            println!("  {} [stdio]", name);
                            println!("    command: {}{}", server.command, args_str);
                        }
                        if let Some(timeout) = server.timeout {
                            println!("    timeout: {}s", timeout);
                        }
                    }
                }
                has_output = true;
            }
        }

        if !has_output {
            println!("No tools configured. Use --config, --dmn, or --images to enable tools.");
        }
        return Ok(());
    }

    // Handle --list-tools
    if args.list_tools {
        let enable_image_tool = args.dmn || args.images;
        let enable_search_tool = args.dmn || args.search;
        let mut all_tool_names: Vec<String> = Vec::new();

        // Add built-in tools
        if enable_image_tool {
            all_tool_names.push(agent::INTERPRET_IMAGE_TOOL_NAME.to_string());
        }
        if enable_search_tool {
            all_tool_names.push(agent::SEARCH_QUERY_TOOL_NAME.to_string());
        }

        // Get MCP tools if config exists
        let mut manager_opt: Option<McpManager> = None;
        if let Some(ref config) = mcp_config {
            if !config.mcp_servers.is_empty() {
                let mut manager = McpManager::new();
                manager.start_servers_background(config, true, args.verbose);
                manager.await_all_servers().await;

                let tools = manager.get_tools();
                for tool in &tools {
                    all_tool_names.push(tool.function.name.clone());
                }
                manager_opt = Some(manager);
            }
        }

        if all_tool_names.is_empty() {
            println!("No tools available. Use --config, --dmn, or --images to enable tools.");
        } else {
            all_tool_names.sort();
            println!("Discovered tools ({}):\n", all_tool_names.len());

            for name in &all_tool_names {
                let is_builtin = name == agent::INTERPRET_IMAGE_TOOL_NAME || name == agent::SEARCH_QUERY_TOOL_NAME;

                if is_builtin {
                    println!("  {} [built-in]", name);
                } else if let Some(ref config) = mcp_config {
                    // Check allowedTools filter (whitelist)
                    let passes_allowed = config.allowed_tools.is_empty() ||
                        config.allowed_tools.iter().any(|p| crate::mcp::tool_matches_pattern(name, p));

                    // Check deniedTools filter (blacklist)
                    let is_denied = !config.denied_tools.is_empty() &&
                        config.denied_tools.iter().any(|p| crate::mcp::tool_matches_pattern(name, p));

                    // Check which agents have access to this tool
                    let mut agent_access: Vec<&str> = Vec::new();
                    for (agent_name, agent_config) in &config.agents {
                        if agent_config.tools.iter().any(|p| crate::mcp::tool_matches_pattern(name, p)) {
                            agent_access.push(agent_name);
                        }
                    }
                    agent_access.sort();

                    // Build display string
                    let filter_status = if is_denied {
                        Some("denied")
                    } else if !config.allowed_tools.is_empty() {
                        Some(if passes_allowed { "allowed" } else { "filtered" })
                    } else {
                        None
                    };

                    let agents_str = if !agent_access.is_empty() {
                        Some(format!("agents: {}", agent_access.join(", ")))
                    } else {
                        None
                    };

                    match (filter_status, &agents_str) {
                        (None, None) => println!("  {}", name),
                        (Some(f), None) => println!("  {} [{}]", name, f),
                        (None, Some(a)) => println!("  {} [{}]", name, a),
                        (Some(f), Some(a)) => println!("  {} [{}, {}]", name, f, a),
                    }
                } else {
                    println!("  {}", name);
                }
            }

            // Summary
            if let Some(ref config) = mcp_config {
                println!();
                if !config.allowed_tools.is_empty() {
                    let allowed_count = all_tool_names.iter().filter(|n| {
                        *n == agent::INTERPRET_IMAGE_TOOL_NAME ||
                        *n == agent::SEARCH_QUERY_TOOL_NAME ||
                        config.allowed_tools.iter().any(|p| crate::mcp::tool_matches_pattern(n, p))
                    }).count();
                    println!("allowedTools: {:?}", config.allowed_tools);
                    println!("  {} of {} tools pass whitelist\n", allowed_count, all_tool_names.len());
                }

                if !config.denied_tools.is_empty() {
                    let denied_count = all_tool_names.iter().filter(|n| {
                        config.denied_tools.iter().any(|p| crate::mcp::tool_matches_pattern(n, p))
                    }).count();
                    println!("deniedTools: {:?}", config.denied_tools);
                    println!("  {} of {} tools blocked by blacklist\n", denied_count, all_tool_names.len());
                }

                if !config.agents.is_empty() {
                    println!("Agents:");
                    let mut agent_names: Vec<_> = config.agents.keys().collect();
                    agent_names.sort();
                    for agent_name in agent_names {
                        if let Some(agent_cfg) = config.agents.get(agent_name) {
                            let tool_count = all_tool_names.iter().filter(|n| {
                                agent_cfg.tools.iter().any(|p| crate::mcp::tool_matches_pattern(n, p))
                            }).count();
                            println!("  {}: {} tools (patterns: {:?})", agent_name, tool_count, agent_cfg.tools);
                        }
                    }
                }
            }
        }

        // Shutdown MCP servers if started
        if let Some(mut manager) = manager_opt {
            manager.shutdown().await?;
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
    let client = Client::new(&provider_info, args.verbose)?;

    // Initialize MCP manager (background startup for faster prompt display)
    let (mut mcp_manager, orchestrator) = if let Some(ref config) = mcp_config {
        let mut manager = McpManager::new();
        // Start servers in background - they'll be awaited when tools are called
        manager.start_servers_background(config, args.silent, args.verbose);

        // Set allowed tools filter if configured
        if !config.allowed_tools.is_empty() {
            manager.set_allowed_tools(config.allowed_tools.clone());
        }

        // Set denied tools filter if configured
        if !config.denied_tools.is_empty() {
            manager.set_denied_tools(config.denied_tools.clone());
        }

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
            args.dmn || args.images,
            args.dmn || args.search,
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
                if manager.has_pending_servers() {
                    let mut names = manager.pending_server_names();
                    display::debug(&format!("Waiting for MCP server(s) to initialize: {}", names.join(", ")), args.verbose);
                    let spinner = display::Spinner::start(&format!(
                        "Starting MCP server{}: {}",
                        if names.len() > 1 { "s" } else { "" },
                        names.join(", ")
                    ));
                    while !names.is_empty() {
                        names = manager.await_next_pending_server().await;
                        if !names.is_empty() {
                            spinner.set_message(&format!(
                                "Starting MCP server{}: {}",
                                if names.len() > 1 { "s" } else { "" },
                                names.join(", ")
                            ));
                        }
                    }
                    spinner.stop().await;
                }
                let server_info = manager.get_server_info();
                display::print_mcp_info(&server_info);
            }

            if args.dmn {
                display::print_dmn_mode();
            }

            // Show agent info if in multi-agent mode
            if let (Some(ref orch), Some(ref name), Some(ref manager)) = (&orchestrator, &agent_name, &mcp_manager) {
                if let Some(agent) = orch.get_agent(name) {
                    // Count tools this agent has access to
                    let all_tools = manager.get_tools();
                    let tools_count = all_tools.iter().filter(|t| {
                        agent.tools.iter().any(|p| crate::mcp::tool_matches_pattern(&t.function.name, p))
                    }).count();
                    display::print_agent_info(name, tools_count, &agent.can_invoke);
                }
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
                None, // No caller for root agent
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

            // Enable compaction in DMN mode by default
            let compaction_config = if args.dmn {
                Some(compact::CompactionConfig::default())
            } else {
                None
            };

            agent::run_agent(
                &client,
                &provider_info.resolved_model,
                &final_prompt,
                args.tool_output_limit,
                mcp_manager.as_mut(),
                args.silent,
                args.verbose,
                &mut conversation_history,
                args.dmn || args.images,
                args.dmn || args.search,
                compaction_config,
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

/// Format agents for display (testable helper)
pub fn format_agents(config: &McpConfig) -> Vec<String> {
    let mut result = Vec::new();
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
            result.push(format!("{}: [{}]{}", name, tools_str, invoke_str));
        }
    }
    result
}

/// Format MCP servers for display (testable helper)
pub fn format_mcp_servers(config: &McpConfig) -> Vec<String> {
    let mut result = Vec::new();
    let mut names: Vec<_> = config.mcp_servers.keys().collect();
    names.sort();
    for name in names {
        if let Some(server) = config.mcp_servers.get(name) {
            if let Some(ref url) = server.url {
                result.push(format!("{} [http] url: {}", name, url));
            } else {
                let args_str = if server.args.is_empty() {
                    String::new()
                } else {
                    format!(" {}", server.args.join(" "))
                };
                result.push(format!("{} [stdio] command: {}{}", name, server.command, args_str));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_list_models() {
        let args = Args::try_parse_from(["eunice", "--list-models"]).unwrap();
        assert!(args.list_models);
        assert!(!args.list_agents);
        assert!(!args.list_tools);
        assert!(!args.list_mcp_servers);
    }

    #[test]
    fn test_args_list_agents() {
        let args = Args::try_parse_from(["eunice", "--list-agents"]).unwrap();
        assert!(args.list_agents);
        assert!(!args.list_models);
        assert!(!args.list_tools);
        assert!(!args.list_mcp_servers);
    }

    #[test]
    fn test_args_list_tools() {
        let args = Args::try_parse_from(["eunice", "--list-tools"]).unwrap();
        assert!(args.list_tools);
        assert!(!args.list_models);
        assert!(!args.list_agents);
        assert!(!args.list_mcp_servers);
    }

    #[test]
    fn test_args_list_mcp_servers() {
        let args = Args::try_parse_from(["eunice", "--list-mcp-servers"]).unwrap();
        assert!(args.list_mcp_servers);
        assert!(!args.list_models);
        assert!(!args.list_agents);
        assert!(!args.list_tools);
    }

    #[test]
    fn test_args_dmn_alias() {
        let args = Args::try_parse_from(["eunice", "--dmn"]).unwrap();
        assert!(args.dmn);
    }

    #[test]
    fn test_args_default_tool_output_limit() {
        let args = Args::try_parse_from(["eunice", "hello"]).unwrap();
        assert_eq!(args.tool_output_limit, 50);
    }

    #[test]
    fn test_format_agents_empty() {
        let config = McpConfig {
            mcp_servers: std::collections::HashMap::new(),
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        };
        let result = format_agents(&config);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_agents_with_config() {
        use crate::models::AgentConfig;
        let mut config = McpConfig {
            mcp_servers: std::collections::HashMap::new(),
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        };
        config.agents.insert("root".to_string(), AgentConfig {
            prompt: "You are root".to_string(),
            mcp_servers: vec![],
            tools: vec!["tool1".to_string()],
            can_invoke: vec!["worker".to_string()],
        });
        config.agents.insert("worker".to_string(), AgentConfig {
            prompt: "You are worker".to_string(),
            mcp_servers: vec![],
            tools: vec!["tool2".to_string(), "tool3".to_string()],
            can_invoke: Vec::new(),
        });
        let result = format_agents(&config);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "root: [tool1] → can invoke: worker");
        assert_eq!(result[1], "worker: [tool2, tool3]");
    }

    #[test]
    fn test_format_mcp_servers_stdio() {
        use crate::models::McpServerConfig;
        let mut config = McpConfig {
            mcp_servers: std::collections::HashMap::new(),
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        };
        config.mcp_servers.insert("shell".to_string(), McpServerConfig {
            command: "mcpz".to_string(),
            args: vec!["server".to_string(), "shell".to_string()],
            url: None,
            timeout: None,
        });
        let result = format_mcp_servers(&config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "shell [stdio] command: mcpz server shell");
    }

    #[test]
    fn test_format_mcp_servers_http() {
        use crate::models::McpServerConfig;
        let mut config = McpConfig {
            mcp_servers: std::collections::HashMap::new(),
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        };
        config.mcp_servers.insert("remote".to_string(), McpServerConfig {
            command: String::new(),
            args: Vec::new(),
            url: Some("http://localhost:3323/mcp".to_string()),
            timeout: None,
        });
        let result = format_mcp_servers(&config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "remote [http] url: http://localhost:3323/mcp");
    }
}
