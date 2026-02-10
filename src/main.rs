mod agent;
mod client;
mod compact;
mod display;
mod display_sink;
mod interactive;
mod models;
mod output_store;
mod provider;
mod skills;
mod tools;
mod tui;
mod usage;
mod webapp;

use crate::client::Client;
use crate::display_sink::create_display_sink;
use crate::models::Message;
use crate::provider::{detect_provider, get_smart_default_model, supports_tools};
use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::Path;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Full LLM context documentation
const LLMS_FULL_TXT: &str = include_str!("../llms-full.txt");

#[derive(Parser)]
#[command(name = "eunice", about = "Agentic CLI runner with OpenAI, Gemini, Claude, and Ollama support", version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"))]
struct Args {
    /// AI model to use
    #[arg(long)]
    model: Option<String>,

    /// System prompt (inline text or file path)
    #[arg(long)]
    prompt: Option<String>,

    /// Positional prompt argument
    prompt_positional: Option<String>,

    /// Interactive chat mode with enhanced terminal interface
    #[arg(long, alias = "tui")]
    chat: bool,

    /// Start web server interface
    #[arg(long)]
    webapp: bool,

    /// Port for webapp server (default: 8811)
    #[arg(long, default_value = "8811")]
    port: u16,

    /// Host for webapp server (default: 0.0.0.0)
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// List available AI models
    #[arg(long)]
    list_models: bool,

    /// List the 4 built-in tools
    #[arg(long)]
    list_tools: bool,

    /// List available skills from ~/.eunice/skills/
    #[arg(long)]
    list_skills: bool,

    /// Output full LLM context documentation
    #[arg(long = "llms-txt")]
    llms_txt: bool,

    /// Update eunice to the latest version
    #[arg(long)]
    update: bool,

    /// Force reinstall even if already up to date (use with --update)
    #[arg(short, long)]
    force: bool,

    /// Uninstall eunice
    #[arg(long)]
    uninstall: bool,
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

/// Update eunice to the latest version from GitHub
fn run_update(force: bool) -> Result<()> {
    use std::process::{Command, Stdio};

    if force {
        println!("Force reinstall requested...");
        println!("Current version: {}", VERSION);
        println!();
    } else {
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
    }

    // Check if cargo is available
    let cargo_check = Command::new("cargo")
        .arg("--version")
        .output();

    if cargo_check.is_err() {
        return Err(anyhow!("cargo is not installed. Please install Rust/Cargo first."));
    }

    // Run cargo install from git with SSH
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
        println!("Update complete!");
        println!("Run 'eunice --version' to verify.");
        Ok(())
    } else {
        Err(anyhow!("Update failed. Check the output above for details."))
    }
}

/// Uninstall eunice
fn run_uninstall() -> Result<()> {
    use std::process::{Command, Stdio};

    println!("Uninstalling eunice...");
    println!();

    // Check if cargo is available
    let cargo_check = Command::new("cargo").arg("--version").output();
    if cargo_check.is_err() {
        return Err(anyhow!("cargo is not installed. Manual removal required."));
    }

    // Run cargo uninstall
    let status = Command::new("cargo")
        .args(["uninstall", "eunice"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if status.success() {
        println!();
        println!("Uninstall complete!");
        println!();
        println!("Note: Configuration files in ~/.eunice/ were preserved.");
        println!("To remove them: rm -rf ~/.eunice");
        Ok(())
    } else {
        Err(anyhow!("Uninstall failed. Check the output above for details."))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --list-models
    if args.list_models {
        display::print_model_list();
        return Ok(());
    }

    // Handle --update
    if args.update {
        return run_update(args.force);
    }

    // Handle --uninstall
    if args.uninstall {
        return run_uninstall();
    }

    // Handle --llms-txt
    if args.llms_txt {
        print!("{}", LLMS_FULL_TXT);
        return Ok(());
    }

    // Handle --list-tools
    if args.list_tools {
        let registry = tools::ToolRegistry::new();
        let tool_specs = registry.get_tools();
        println!("Built-in tools ({}):\n", tool_specs.len());
        for tool in tool_specs {
            println!("  {}", tool.function.name);
            println!("    {}\n", tool.function.description);
        }
        return Ok(());
    }

    // Handle --list-skills
    if args.list_skills {
        // Ensure skills are installed first
        let _ = skills::ensure_default_skills();
        let output = skills::list_all_skills().await?;
        println!("{}", output);
        return Ok(());
    }

    // Validate conflicting arguments
    if args.webapp && args.chat {
        return Err(anyhow!("--webapp and --chat cannot be used together"));
    }
    if args.chat && !atty::is(atty::Stream::Stdin) {
        return Err(anyhow!("--chat requires an interactive terminal (TTY)"));
    }

    // Ensure default skills are installed
    if let Err(e) = skills::ensure_default_skills() {
        eprintln!("Warning: failed to install default skills: {}", e);
    }

    // Resolve prompt
    let prompt = resolve_prompt(&args)?;

    // Determine if we need TUI mode
    let use_tui = args.chat || (prompt.is_none() && atty::is(atty::Stream::Stdin));

    // Select model
    let model = match &args.model {
        Some(m) => m.clone(),
        None => get_smart_default_model()?,
    };

    // Detect provider and check tool support
    let provider_info = detect_provider(&model)?;

    if !supports_tools(&provider_info.provider, &model) {
        eprintln!("Warning: Model '{}' may not support function calling.", model);
        eprintln!("Running in text-only mode (no Bash/Read/Write/Skill tools available).");
        eprintln!("Tip: For full tool support, try: llama3.1, qwen2.5, or mistral-nemo\n");
    }

    // Create client
    let client = Client::new(&provider_info)?;

    // Webapp mode
    if args.webapp {
        let webapp_config = models::WebappConfig {
            host: args.host.clone(),
            port: args.port,
        };
        return webapp::run_server(
            webapp_config,
            client,
            provider_info,
        ).await;
    }

    // TUI mode
    if use_tui {
        return tui::run_tui_mode(
            &client,
            &provider_info,
            prompt.as_deref(),
        ).await;
    }

    // Single-shot mode
    let prompt = prompt.unwrap();

    // Show model info
    display::print_model_info(&provider_info.resolved_model, &provider_info.provider);

    // Create display sink
    let display = create_display_sink();

    // Create tool registry
    let tool_registry = tools::ToolRegistry::new();

    // Output store for truncating large tool outputs
    let mut output_store = output_store::OutputStore::new();

    // Conversation history
    let mut conversation_history: Vec<Message> = Vec::new();

    // Run agent
    agent::run_agent(
        &client,
        &provider_info.resolved_model,
        &prompt,
        50, // tool_output_limit
        &tool_registry,
        display,
        &mut conversation_history,
        None, // compaction_config
        Some(&mut output_store),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_list_models() {
        let args = Args::try_parse_from(["eunice", "--list-models"]).unwrap();
        assert!(args.list_models);
    }

    #[test]
    fn test_args_list_tools() {
        let args = Args::try_parse_from(["eunice", "--list-tools"]).unwrap();
        assert!(args.list_tools);
    }

    #[test]
    fn test_args_chat() {
        let args = Args::try_parse_from(["eunice", "--chat"]).unwrap();
        assert!(args.chat);
    }

    #[test]
    fn test_args_webapp() {
        let args = Args::try_parse_from(["eunice", "--webapp"]).unwrap();
        assert!(args.webapp);
    }

    #[test]
    fn test_args_prompt() {
        let args = Args::try_parse_from(["eunice", "--prompt", "test prompt"]).unwrap();
        assert_eq!(args.prompt, Some("test prompt".to_string()));
    }

    #[test]
    fn test_args_positional_prompt() {
        let args = Args::try_parse_from(["eunice", "hello world"]).unwrap();
        assert_eq!(args.prompt_positional, Some("hello world".to_string()));
    }
}
