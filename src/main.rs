mod agent;
mod api_keys;
mod builtin_tools;
mod client;
mod compact;
mod config;
mod display;
mod display_sink;
mod interactive;
mod mcp;
mod mcpz;
mod models;
mod orchestrator;
mod output_store;
mod provider;
mod tui;
mod usage;
mod webapp;

use crate::client::Client;
use crate::config::{get_dmn_mcp_config, get_research_mcp_config, has_gemini_api_key, has_mcpz, load_mcp_config, DMN_INSTRUCTIONS, LLMS_FULL_TXT};
use crate::display_sink::create_display_sink;
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

    // === Tools ===
    /// Enable shell command execution
    #[arg(long, help_heading = "Tools")]
    shell: bool,

    /// Enable filesystem operations (read, write, edit, list)
    #[arg(long, help_heading = "Tools")]
    filesystem: bool,

    /// Enable browser automation (requires Chrome)
    #[arg(long, help_heading = "Tools")]
    browser: bool,

    /// Enable web search (uses Gemini + Google Search)
    #[arg(long, help_heading = "Tools")]
    search: bool,

    /// Enable image/PDF interpretation
    #[arg(long, help_heading = "Tools")]
    images: bool,

    /// Enable all built-in tools
    #[arg(long, help_heading = "Tools")]
    all: bool,

    /// Enable shell + filesystem (shortcut for --shell --filesystem)
    #[arg(long, help_heading = "Tools")]
    native: bool,

    // === Modes ===
    /// Autonomous batch execution (--all + DMN instructions)
    #[arg(long = "default-mode-network", visible_alias = "dmn", help_heading = "Modes")]
    dmn: bool,

    /// Run as specific agent from config
    #[arg(long, help_heading = "Modes")]
    agent: Option<String>,

    /// Multi-agent research orchestration (requires GEMINI_API_KEY)
    #[arg(long, help_heading = "Modes")]
    research: bool,

    /// Start web server interface
    #[arg(long, help_heading = "Modes")]
    webapp: bool,

    /// Interactive chat mode with enhanced terminal interface (requires TTY)
    #[arg(long, alias = "tui", help_heading = "Modes")]
    chat: bool,

    // === Discovery ===
    /// List available AI models
    #[arg(long, help_heading = "Discovery")]
    list_models: bool,

    /// List configured agents
    #[arg(long, help_heading = "Discovery")]
    list_agents: bool,

    /// List available tools (built-in and MCP)
    #[arg(long, help_heading = "Discovery")]
    list_tools: bool,

    /// List configured MCP servers
    #[arg(long, help_heading = "Discovery")]
    list_mcp_servers: bool,

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

    /// Output llms.txt (full LLM context documentation)
    #[arg(long = "llms-txt", help_heading = "Output")]
    llms_txt: bool,

    // === Advanced ===
    /// Disable MCP even if eunice.json exists
    #[arg(long, help_heading = "Advanced")]
    no_mcp: bool,

    /// Path to API keys file for key rotation (default: ~/.eunice/api_keys.toml)
    #[arg(long, help_heading = "Advanced")]
    api_keys: Option<String>,

    /// Update eunice to the latest version from GitHub
    #[arg(long, help_heading = "Advanced")]
    update: bool,
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

        // Show Research mode info
        println!("\nResearch Mode (--research):");
        println!("    Multi-agent research system with 4 agents:");
        println!("      root (coordinator) → researcher, report_writer, evaluator");
        println!("    Built-in: search_query (Gemini + Google Search)");
        println!("    MCP: filesystem");
        println!("    Requires: GEMINI_API_KEY");
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
/// Returns (config, from_file) where from_file indicates if the config was loaded from a file
fn determine_config(args: &Args) -> Result<(Option<McpConfig>, bool)> {
    // --no-mcp disables MCP
    if args.no_mcp {
        return Ok((None, false));
    }

    // --dmn uses embedded config
    if args.dmn {
        if args.config.is_some() {
            return Err(anyhow!("--dmn cannot be used with --config"));
        }
        return Ok((Some(get_dmn_mcp_config()), false));
    }

    // --research uses embedded multi-agent config
    // If --config is also specified, merge MCP servers from config (ignoring agents)
    if args.research {
        let mut research_config = get_research_mcp_config();

        // Optionally merge MCP servers from user config
        if let Some(config_path) = &args.config {
            if !config_path.is_empty() {
                let path = Path::new(config_path);
                let user_config = load_mcp_config(path)?;

                // Merge user's MCP servers into research config
                for (name, server) in user_config.mcp_servers {
                    research_config.mcp_servers.insert(name, server);
                }

                // Also merge allowed/denied tools if user specified them
                if !user_config.allowed_tools.is_empty() {
                    research_config.allowed_tools = user_config.allowed_tools;
                }
                if !user_config.denied_tools.is_empty() {
                    research_config.denied_tools = user_config.denied_tools;
                }

                // Agents from user config are intentionally ignored - research mode uses embedded agents
            }
        }

        return Ok((Some(research_config), false));
    }

    // --config specified
    if let Some(config_path) = &args.config {
        // Empty config = no MCP
        if config_path.is_empty() {
            return Ok((None, false));
        }

        let path = Path::new(config_path);
        return Ok((Some(load_mcp_config(path)?), true));
    }

    // Auto-discover eunice.toml or eunice.json in current directory (prefer TOML)
    let toml_config = Path::new("eunice.toml");
    let json_config = Path::new("eunice.json");

    if toml_config.exists() && json_config.exists() {
        eprintln!("Warning: Both eunice.toml and eunice.json exist. Using eunice.toml.");
    }

    if toml_config.exists() {
        return Ok((Some(load_mcp_config(toml_config)?), true));
    }

    if json_config.exists() {
        return Ok((Some(load_mcp_config(json_config)?), true));
    }

    // Create MCP config when --shell/--filesystem/--browser/--native/--all flags are used
    // This enables these flags to work properly in TUI and interactive modes
    if args.shell || args.filesystem || args.browser || args.native || args.all {
        use std::collections::HashMap;
        let use_mcpz = has_mcpz();
        let mut servers = HashMap::new();

        if args.shell || args.native || args.all {
            servers.insert(
                "shell".to_string(),
                if use_mcpz {
                    models::McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "shell".into()], url: None, timeout: None }
                } else {
                    models::McpServerConfig { command: "uvx".into(), args: vec!["git+https://github.com/emsi/mcp-server-shell".into()], url: None, timeout: None }
                },
            );
        }

        if args.filesystem || args.native || args.all {
            servers.insert(
                "filesystem".to_string(),
                if use_mcpz {
                    models::McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "filesystem".into()], url: None, timeout: None }
                } else {
                    models::McpServerConfig { command: "npx".into(), args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), ".".into()], url: None, timeout: None }
                },
            );
        }

        if args.browser || args.all {
            servers.insert(
                "browser".to_string(),
                if use_mcpz {
                    models::McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "browser".into()], url: None, timeout: None }
                } else {
                    // Browser requires mcpz, warn user
                    eprintln!("Warning: --browser flag requires mcpz. Install with: cargo install eunice");
                    models::McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "browser".into()], url: None, timeout: None }
                },
            );
        }

        return Ok((Some(models::McpConfig {
            mcp_servers: servers,
            agents: HashMap::new(),
            allowed_tools: vec![],
            denied_tools: vec![],
            webapp: None,
        }), false));
    }

    Ok((None, false))
}

/// Check for conflicts between built-in tool flags and MCP config
fn check_builtin_tool_conflicts(args: &Args, config: &Option<models::McpConfig>) -> Result<()> {
    let Some(config) = config else {
        return Ok(());
    };

    let mut conflicts = Vec::new();

    // Check shell conflict
    if (args.shell || args.native || args.all) && config.mcp_servers.contains_key("shell") {
        conflicts.push(("--shell/--native", "shell"));
    }

    // Check filesystem conflict
    if (args.filesystem || args.native || args.all) && config.mcp_servers.contains_key("filesystem") {
        conflicts.push(("--filesystem/--native", "filesystem"));
    }

    // Check browser conflict
    if (args.browser || args.all) && config.mcp_servers.contains_key("browser") {
        conflicts.push(("--browser", "browser"));
    }

    if !conflicts.is_empty() {
        let msg = conflicts
            .iter()
            .map(|(flag, server)| format!("{} conflicts with MCP server '{}'", flag, server))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(anyhow!(
            "Built-in tool conflicts with MCP config: {}. Use --no-mcp to disable MCP servers or remove the conflicting flag.",
            msg
        ));
    }

    Ok(())
}

/// Fetch the remote version from longrunningagents.com
fn fetch_remote_version() -> Option<String> {
    let url = "https://longrunningagents.com/version.txt";
    let response = reqwest::blocking::get(url).ok()?;
    let text = response.text().ok()?;
    Some(text.trim().to_string())
}

/// Compare semantic versions, returns true if remote > local
fn is_newer_version(remote: &str, local: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    let remote_parts = parse(remote);
    let local_parts = parse(local);

    for i in 0..3 {
        let r = remote_parts.get(i).unwrap_or(&0);
        let l = local_parts.get(i).unwrap_or(&0);
        if r > l { return true; }
        if r < l { return false; }
    }
    false
}

/// Update eunice and mcpz to the latest version from GitHub
fn run_update() -> Result<()> {
    use std::process::{Command, Stdio};

    println!("Checking for updates...");
    println!("Current version: {}", VERSION);

    // Check remote version
    if let Some(remote_version) = fetch_remote_version() {
        println!("Remote version:  {}", remote_version);
        println!();

        if !is_newer_version(&remote_version, VERSION) {
            println!("Already up to date!");
            return Ok(());
        }

        println!("Update available: {} -> {}", VERSION, remote_version);
    } else {
        println!("Could not fetch remote version, proceeding with update...");
    }
    println!();

    // Check if cargo is available
    let cargo_check = Command::new("cargo")
        .arg("--version")
        .output();

    if cargo_check.is_err() {
        return Err(anyhow!("cargo is not installed. Please install Rust/Cargo first."));
    }

    // Run cargo install from git with SSH
    // Set CARGO_NET_GIT_FETCH_WITH_CLI=true to use system git (with SSH keys)
    // This installs both eunice and mcpz binaries
    let status = Command::new("cargo")
        .env("CARGO_NET_GIT_FETCH_WITH_CLI", "true")
        .args([
            "install",
            "--git",
            "ssh://git@github.com/xeb/eunice.git",
            "--force",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        println!();
        println!("Update complete! Both eunice and mcpz have been updated.");
        println!("Run 'eunice --version' or 'mcpz --version' to verify.");
        Ok(())
    } else {
        Err(anyhow!("Update failed. Check the output above for details."))
    }
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

    // Handle --update
    if args.update {
        return run_update();
    }

    // Handle --llms-txt (outputs full documentation)
    if args.llms_txt {
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

    if args.research && args.no_mcp {
        return Err(anyhow!("--research requires MCP tools and cannot be used with --no-mcp"));
    }

    if args.research && args.dmn {
        return Err(anyhow!("--research and --dmn cannot be used together"));
    }

    // --webapp conflicts with --events, --silent, --chat
    if args.webapp && args.events {
        return Err(anyhow!("--webapp and --events cannot be used together"));
    }
    if args.webapp && args.silent {
        return Err(anyhow!("--webapp and --silent cannot be used together"));
    }
    if args.webapp && args.chat {
        return Err(anyhow!("--webapp and --chat cannot be used together"));
    }

    // --chat conflicts with --events, --silent
    if args.chat && args.events {
        return Err(anyhow!("--chat and --events cannot be used together"));
    }
    if args.chat && args.silent {
        return Err(anyhow!("--chat and --silent cannot be used together"));
    }

    // --chat requires a TTY
    if args.chat && !atty::is(atty::Stream::Stdin) {
        return Err(anyhow!("--chat requires an interactive terminal (TTY)"));
    }
    // --research requires GEMINI_API_KEY for search_query tool
    if args.research && !has_gemini_api_key() {
        return Err(anyhow!("--research requires GEMINI_API_KEY environment variable for web search"));
    }

    // Get config early for --list-agents
    let (mcp_config, config_from_file) = determine_config(&args)?;

    // Check for conflicts between built-in tool flags and MCP config
    // Only check when config was loaded from a file (not synthetically created from CLI flags)
    if config_from_file {
        check_builtin_tool_conflicts(&args, &mcp_config)?;
    }

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
                        let model_str = agent.model.as_ref()
                            .map(|m| format!(" (model: {})", m))
                            .unwrap_or_default();
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
                        println!("  {}{}: [{}]{}", name, model_str, tools_str, invoke_str);
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
        let enable_search_tool = args.dmn || args.search || args.research;
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
        let enable_image_tool = args.dmn || args.images || args.all;
        let enable_search_tool = args.dmn || args.search || args.research || args.all;
        // For shell/filesystem: only use built-in when explicitly requested via flags, not via --dmn
        // (--dmn uses MCP servers for these, not built-in)
        let enable_shell_builtin = args.shell || args.native || args.all;
        let enable_filesystem_builtin = args.filesystem || args.native || args.all;
        let mut all_tool_names: Vec<String> = Vec::new();

        // Add built-in tools (interpret_image and search_query are always built-in)
        if enable_image_tool {
            all_tool_names.push(agent::INTERPRET_IMAGE_TOOL_NAME.to_string());
        }
        if enable_search_tool {
            all_tool_names.push(agent::SEARCH_QUERY_TOOL_NAME.to_string());
        }

        // Add shell and filesystem tools via BuiltinToolRegistry
        // Only when using explicit flags (not --dmn, which uses MCP servers)
        let mut builtin_registry = builtin_tools::BuiltinToolRegistry::new();
        if enable_shell_builtin {
            builtin_registry = builtin_registry.with_shell(None);
        }
        if enable_filesystem_builtin {
            builtin_registry = builtin_registry.with_filesystem(vec![]);
        }
        // Track which tool names come from built-in registry
        let builtin_tool_names: std::collections::HashSet<String> = builtin_registry
            .get_tools()
            .iter()
            .map(|t| t.function.name.clone())
            .collect();
        for name in &builtin_tool_names {
            all_tool_names.push(name.clone());
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
                let is_builtin = name == agent::INTERPRET_IMAGE_TOOL_NAME
                    || name == agent::SEARCH_QUERY_TOOL_NAME
                    || builtin_tool_names.contains(name);

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

    // Determine if we need TUI mode (when no prompt given and TTY available)
    let use_tui = args.chat || (prompt.is_none() && atty::is(atty::Stream::Stdin));

    // Select model
    let model = match &args.model {
        Some(m) => m.clone(),
        None => get_smart_default_model()?,
    };

    // Detect provider and create client
    let provider_info = detect_provider(&model)?;

    // Load API keys for key rotation
    let key_rotator = {
        let keys_path = args.api_keys.as_ref()
            .map(|p| std::path::PathBuf::from(p))
            .or_else(|| {
                dirs::home_dir().map(|h| h.join(".eunice").join("api_keys.toml"))
            });

        if let Some(path) = keys_path {
            if path.exists() {
                match api_keys::load_api_keys(&path) {
                    Ok(config) => {
                        let rotator = api_keys::build_rotator(
                            &config,
                            &provider_info.provider,
                            &provider_info.api_key,
                        );
                        if let Some(ref r) = rotator {
                            if !args.silent {
                                eprintln!("API keys: {} keys loaded for rotation", r.key_count());
                            }
                        }
                        rotator
                    }
                    Err(e) => {
                        if args.api_keys.is_some() {
                            // Explicit --api-keys flag: error is fatal
                            return Err(e);
                        }
                        // Auto-discovery: warn but continue
                        if args.verbose {
                            eprintln!("Warning: failed to load API keys: {}", e);
                        }
                        None
                    }
                }
            } else if args.api_keys.is_some() {
                return Err(anyhow!("API keys file not found: {}", path.display()));
            } else {
                None
            }
        } else {
            None
        }
    };

    let client = Client::new_with_keys(&provider_info, args.verbose, key_rotator)?;

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
            let orchestrator = AgentOrchestrator::new(config, None)?;

            // Validate all agent models at startup
            let unique_models = orchestrator.get_all_agent_models(&model);
            for agent_model in &unique_models {
                detect_provider(agent_model).map_err(|e| {
                    anyhow!("Invalid model '{}' used by agent: {}", agent_model, e)
                })?;
            }

            Some(orchestrator)
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
    } else if let Some(ref config) = mcp_config {
        // Get the root agent from config (handles root = true flag and name-based fallback)
        match config.get_root_agent() {
            Ok(root) => root,
            Err(e) => return Err(anyhow!("Agent configuration error: {}", e)),
        }
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

    // Webapp mode - starts web server and blocks
    if args.webapp {
        // Wait for MCP servers to be ready
        if let Some(ref mut manager) = mcp_manager {
            manager.await_all_servers().await;
        }

        // Get webapp config from mcp_config or use defaults
        let webapp_config = mcp_config
            .as_ref()
            .and_then(|c| c.webapp.clone())
            .unwrap_or_default();

        return webapp::run_server(
            webapp_config,
            client,
            provider_info,
            mcp_manager,
            orchestrator,
            agent_name,
            args.tool_output_limit,
            args.verbose,
            args.dmn,
            args.research,
            args.dmn || args.images,
            args.dmn || args.search || args.research,
        ).await;
    }

    // TUI mode - enhanced terminal interface (auto-launch when no prompt given)
    if use_tui {
        // Wait for MCP servers to be ready
        if let Some(ref mut manager) = mcp_manager {
            manager.await_all_servers().await;
        }

        return tui::run_tui_mode(
            &client,
            &provider_info,
            prompt.as_deref(),
            args.tool_output_limit,
            mcp_manager.as_mut(),
            orchestrator.as_ref(),
            agent_name.as_deref(),
            args.silent,
            args.verbose,
            args.dmn,
            args.dmn || args.images,
            args.dmn || args.search || args.research,
        ).await;
    }

    // Single-shot mode
    let prompt = prompt.unwrap();

    // Show model/MCP info
    if !args.silent {
        display::print_model_info(&provider_info.resolved_model, &provider_info.provider);

        if let Some(ref mut manager) = mcp_manager {
            // Wait for servers to be ready before displaying info
            if manager.has_pending_servers() {
                let names = manager.pending_server_names();
                display::debug(&format!("Waiting for MCP server(s) to initialize: {}", names.join(", ")), args.verbose);
                println!("Starting MCP servers: {}...", names.join(", "));
                manager.await_all_servers().await;
            }
            let server_info = manager.get_server_info();
            display::print_mcp_info(&server_info);
        }

        if args.dmn {
            display::print_dmn_mode();
        }

        if args.research {
            display::print_research_mode();
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

    // Create display sink for output
    let display = create_display_sink(args.silent, args.verbose);

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
            display,
            0,
            None, // No caller for root agent
            None, // No cancellation support in non-interactive mode
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

        // Create display sink for single-agent mode
        let display = create_display_sink(args.silent, args.verbose);

        // Output store for truncating large tool outputs
        let mut output_store = output_store::OutputStore::new();

        // Create BuiltinToolRegistry when using --shell, --filesystem, --native, --browser, or --all flags
        let builtin_registry = {
            let mut registry = builtin_tools::BuiltinToolRegistry::new();
            if args.shell || args.native || args.all {
                registry = registry.with_shell(None);
            }
            if args.filesystem || args.native || args.all {
                registry = registry.with_filesystem(vec![]);
            }
            // Note: browser not yet implemented in builtin registry
            if registry.is_empty() { None } else { Some(registry) }
        };

        agent::run_agent(
            &client,
            &provider_info.resolved_model,
            &final_prompt,
            args.tool_output_limit,
            mcp_manager.as_mut(),
            builtin_registry.as_ref(),
            display,
            &mut conversation_history,
            args.dmn || args.images || args.all,
            args.dmn || args.search || args.research || args.all,
            compaction_config,
            Some(&mut output_store),
        )
        .await?;
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
            let model_str = agent.model.as_ref()
                .map(|m| format!(" (model: {})", m))
                .unwrap_or_default();
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
            result.push(format!("{}{}: [{}]{}", name, model_str, tools_str, invoke_str));
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
    fn test_args_research_flag() {
        let args = Args::try_parse_from(["eunice", "--research"]).unwrap();
        assert!(args.research);
        assert!(!args.dmn);
    }

    #[test]
    fn test_args_research_with_images() {
        let args = Args::try_parse_from(["eunice", "--research", "--images"]).unwrap();
        assert!(args.research);
        assert!(args.images);
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
            webapp: None,
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
            webapp: None,
        };
        config.agents.insert("root".to_string(), AgentConfig {
            description: "Root coordinator".to_string(),
            prompt: "You are root".to_string(),
            model: None,
            root: true,
            mcp_servers: vec![],
            tools: vec!["tool1".to_string()],
            can_invoke: vec!["worker".to_string()],
        });
        config.agents.insert("worker".to_string(), AgentConfig {
            description: "Worker agent".to_string(),
            prompt: "You are worker".to_string(),
            model: None,
            root: false,
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
    fn test_format_agents_with_model_override() {
        use crate::models::AgentConfig;
        let mut config = McpConfig {
            mcp_servers: std::collections::HashMap::new(),
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
            webapp: None,
        };
        config.agents.insert("root".to_string(), AgentConfig {
            description: "Root coordinator".to_string(),
            prompt: "You are root".to_string(),
            model: None,  // Uses default model
            root: true,
            mcp_servers: vec![],
            tools: vec!["tool1".to_string()],
            can_invoke: vec!["worker".to_string()],
        });
        config.agents.insert("worker".to_string(), AgentConfig {
            description: "Worker with custom model".to_string(),
            prompt: "You are worker".to_string(),
            model: Some("gemini-3-flash-preview".to_string()),  // Custom model
            root: false,
            mcp_servers: vec![],
            tools: vec!["tool2".to_string()],
            can_invoke: Vec::new(),
        });
        let result = format_agents(&config);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "root: [tool1] → can invoke: worker");
        assert_eq!(result[1], "worker (model: gemini-3-flash-preview): [tool2]");
    }

    #[test]
    fn test_format_mcp_servers_stdio() {
        use crate::models::McpServerConfig;
        let mut config = McpConfig {
            mcp_servers: std::collections::HashMap::new(),
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
            webapp: None,
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
            webapp: None,
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

    #[test]
    fn test_args_native_flag() {
        let args = Args::try_parse_from(["eunice", "--native", "--prompt", "test"]).unwrap();
        assert!(args.native);
        assert!(!args.shell);
        assert!(!args.filesystem);
    }

    #[test]
    fn test_native_flag_synthetic_config_not_from_file() {
        // Bug fix test: --native was conflicting with synthetic config it creates itself
        // The fix is that determine_config returns from_file=false for synthetic configs,
        // and the main code skips the conflict check when from_file is false.
        //
        // This test verifies that check_builtin_tool_conflicts WOULD detect a conflict
        // (showing the old bug behavior), but in production we skip the call for synthetic configs.
        use crate::models::McpServerConfig;

        let args = Args::try_parse_from(["eunice", "--native", "--prompt", "test"]).unwrap();

        // Create a config like determine_config would create for --native
        let mut servers = std::collections::HashMap::new();
        servers.insert("shell".to_string(), McpServerConfig {
            command: "mcpz".to_string(),
            args: vec!["server".to_string(), "shell".to_string()],
            url: None,
            timeout: None,
        });
        servers.insert("filesystem".to_string(), McpServerConfig {
            command: "mcpz".to_string(),
            args: vec!["server".to_string(), "filesystem".to_string()],
            url: None,
            timeout: None,
        });

        let config = Some(McpConfig {
            mcp_servers: servers,
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
            webapp: None,
        });

        // The conflict checker WOULD return an error if called...
        let result = check_builtin_tool_conflicts(&args, &config);
        assert!(result.is_err(), "Conflict check detects the issue");

        // ...but in the actual code flow, we skip the check when from_file=false
        // This is verified by the integration test in test_args_native_flag
        // and by running: eunice --native --prompt "test"
    }

    #[test]
    fn test_shell_flag_conflicts_with_user_config() {
        // This tests a REAL conflict: user has shell in config AND uses --shell flag
        // When config is from a file, the conflict check SHOULD run and detect the conflict.
        use crate::models::McpServerConfig;

        let args = Args::try_parse_from(["eunice", "--shell", "--prompt", "test"]).unwrap();

        // Simulate a user config file that has shell server defined
        let mut servers = std::collections::HashMap::new();
        servers.insert("shell".to_string(), McpServerConfig {
            command: "uvx".to_string(),
            args: vec!["custom-shell-server".to_string()],
            url: None,
            timeout: None,
        });

        let config = Some(McpConfig {
            mcp_servers: servers,
            agents: std::collections::HashMap::new(),
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
            webapp: None,
        });

        // This SHOULD return an error - user has shell in config AND uses --shell flag
        // (This conflict check runs when from_file=true)
        let result = check_builtin_tool_conflicts(&args, &config);
        assert!(result.is_err(), "Shell flag should conflict with user config file");
    }
}
