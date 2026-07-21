mod agent;
mod agents;
mod client;
mod compact;
mod daemon;
mod display;
mod display_sink;
mod gemmad;
mod interactive;
mod key_rotation;
mod local;
mod models;
mod output_store;
mod provider;
mod skills;
mod theme;
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

    /// Shorthand for --model=gemma4:31b (local Gemma 4 31B + MTP)
    #[arg(long)]
    gemma: bool,

    /// Use the already-running gemmad daemon (local gemma-4-12b); error if unreachable
    #[arg(long)]
    gemmad: bool,

    /// No-op; kept for compatibility (gemmad is never used implicitly)
    #[arg(long)]
    no_gemmad: bool,

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

    /// Enable debug output for API calls
    #[arg(long)]
    debug: bool,

    /// Download a local model (e.g., hf:gemma4:e4b)
    #[arg(long)]
    download: Option<String>,

    /// List downloaded local models
    #[arg(long)]
    local_models: bool,

    /// Remove a downloaded local model (e.g., gemma4:e4b)
    #[arg(long)]
    remove_model: Option<String>,

    /// Start gemma4-server for a local model (e.g., hf:gemma4:e4b)
    #[arg(long)]
    serve: Option<String>,

    /// Force a clean rebuild of the gemma4-mtp server binary (for --model gemma4:31b)
    #[arg(long)]
    rebuild_gemma4_mtp: bool,

    /// Disable webapp session persistence (sessions.db); sessions live in memory only
    #[arg(long)]
    no_persist: bool,

    /// Path to an agents.toml defining scheduled long-running agents (webapp mode)
    #[arg(long)]
    agents: Option<String>,

    /// Install eunice --webapp as a systemd user service
    #[arg(long)]
    install: bool,

    /// Remove the systemd user service installed by --install
    #[arg(long)]
    uninstall_service: bool,
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

    // Validated before the early-return dispatch below, which would otherwise
    // consume --install/--uninstall-service before these checks could fire.
    if args.install && args.uninstall_service {
        return Err(anyhow!("--install and --uninstall-service cannot be used together"));
    }
    if args.install && args.uninstall {
        return Err(anyhow!("--install and --uninstall cannot be used together"));
    }
    if args.agents.is_some() && !args.webapp && !args.install {
        return Err(anyhow!("--agents requires --webapp or --install"));
    }

    // Handle --uninstall-service
    if args.uninstall_service {
        return daemon::run_uninstall_service();
    }

    // Handle --install
    if args.install {
        return daemon::run_install(&daemon::InstallOptions {
            port: args.port,
            host: args.host.clone(),
            agents_file: args.agents.clone(),
            model: args.model.clone(),
            prompt: args.prompt.clone(),
            no_persist: args.no_persist,
        });
    }

    // Handle --list-models
    if args.list_models {
        // Use block_in_place to allow blocking HTTP calls for Ollama check
        tokio::task::block_in_place(|| {
            display::print_model_list();
        });
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

    // Handle --rebuild-gemma4-mtp
    if args.rebuild_gemma4_mtp {
        let path = local::rebuild_mtp_server().await?;
        eprintln!("Rebuilt MTP server: {}", path.display());
        return Ok(());
    }

    // Handle --download hf:gemma4:*  (also gemma4:31b without prefix)
    if let Some(ref dl_model) = args.download {
        let alias = dl_model.strip_prefix("hf:").unwrap_or(dl_model);
        let info = local::resolve_hf_alias(alias);
        if info.mtp {
            // Validate first, then fetch both GGUFs and pre-warm the build.
            let pf = local::preflight_gemma4_mtp(&info);
            local::print_preflight(&pf);
            if !pf.ok() {
                return Err(anyhow!(
                    "gemma4:31b prerequisites not met — fix the ✗ items above and retry."
                ));
            }
            local::download_model_full(alias).await?;
            let arch = pf.cuda_arch.clone().unwrap_or_else(|| "89".to_string());
            local::ensure_mtp_server(&arch, false).await?;
        } else {
            local::download_model(alias).await?;
        }
        return Ok(());
    }

    // Handle --local-models
    if args.local_models {
        local::print_local_models()?;
        return Ok(());
    }

    // Handle --remove-model
    if let Some(ref rm_model) = args.remove_model {
        local::remove_model(rm_model)?;
        return Ok(());
    }

    // Handle --serve hf:gemma4:*  (and gemma4:31b — auto-downloads/builds + MTP flags as needed)
    if let Some(ref serve_model) = args.serve {
        let alias = serve_model.strip_prefix("hf:").unwrap_or(serve_model);
        let port = args.port;
        let actual_port = if port == 8811 { local::DEFAULT_PORT } else { port };
        let (mut child, _path) = local::setup_local_model_on_port(alias, actual_port).await?;
        eprintln!("server ready at http://127.0.0.1:{}/v1/", actual_port);
        eprintln!("Press Ctrl+C to stop.");
        // Wait for the server to exit (blocks until Ctrl+C)
        let _ = child.wait();
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

    // Select model. The smart default is the global default; --gemmad selects the
    // local daemon, --gemma still builds the 31B MTP server. Only --gemmad probes,
    // so a bare invocation no longer pays the daemon round-trip on every startup.
    let need_probe = args.gemmad;
    let gemmad_up = if need_probe { gemmad::is_available().await } else { false };
    let choice = gemmad::decide_model(
        args.gemma,
        args.gemmad,
        args.no_gemmad,
        args.model.as_deref(),
        gemmad_up,
    )?;
    let used_gemmad = matches!(choice, gemmad::ModelChoice::Gemmad);
    let model = match choice {
        gemmad::ModelChoice::Explicit(m) => m,
        gemmad::ModelChoice::Gemmad => gemmad::live_model_id()
            .await
            .unwrap_or_else(gemmad::model_id),
        gemmad::ModelChoice::Gemma31b => "gemma4:31b".to_string(),
        gemmad::ModelChoice::SmartDefault => get_smart_default_model()?,
    };
    if used_gemmad {
        eprintln!(
            "Using local gemmad ({}) at {}:{}",
            model,
            gemmad::host(),
            gemmad::port()
        );
    }

    // Detect provider and check tool support
    let provider_info = detect_provider(&model)?;

    // If local provider, start gemma4-server
    let mut _local_server: Option<std::process::Child> = if provider_info.provider == models::Provider::Local {
        let alias = model.strip_prefix("hf:").unwrap_or(&model);
        let (child, _path) = local::setup_local_model(alias).await?;
        Some(child)
    } else {
        None
    };

    if !supports_tools(&provider_info.provider, &model) {
        eprintln!("Warning: Model '{}' may not support function calling.", model);
        eprintln!("Running in text-only mode (no Bash/Read/Write/Skill tools available).");
        eprintln!("Tip: For full tool support, try: llama3.1, qwen2.5, or mistral-nemo\n");
    }

    // Create client
    let mut client = Client::new(&provider_info)?;
    if args.debug {
        client.set_debug(true);
        eprintln!("[DEBUG] Debug mode enabled");
        eprintln!("[DEBUG] Model: {}, Provider: {}", model, provider_info.provider);
        eprintln!("[DEBUG] Base URL: {}", provider_info.base_url);
    }

    // Webapp mode
    if args.webapp {
        let webapp_config = models::WebappConfig {
            host: args.host.clone(),
            port: args.port,
            persist: !args.no_persist,
        };
        let agents_config = match args.agents {
            Some(ref file) => Some(agents::load_agents(Path::new(file))?),
            None => None,
        };
        let result = webapp::run_server(
            webapp_config,
            client,
            provider_info,
            prompt.clone(),
            agents_config,
        ).await;
        if let Some(ref mut child) = _local_server {
            let _ = child.kill();
            let _ = child.wait();
        }
        return result;
    }

    // TUI mode
    if use_tui {
        let result = tui::run_tui_mode(
            &client,
            &provider_info,
            prompt.as_deref(),
        ).await;
        if let Some(ref mut child) = _local_server {
            let _ = child.kill();
            let _ = child.wait();
        }
        return result;
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

    // Kill local server if running
    if let Some(ref mut child) = _local_server {
        let _ = child.kill();
        let _ = child.wait();
    }

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

    #[test]
    fn test_args_gemma_flag() {
        let args = Args::try_parse_from(["eunice", "--gemma", "hi"]).unwrap();
        assert!(args.gemma);
        assert_eq!(args.model, None);
        assert_eq!(args.prompt_positional, Some("hi".to_string()));
    }

    #[test]
    fn test_args_gemma_default_false() {
        let args = Args::try_parse_from(["eunice", "hi"]).unwrap();
        assert!(!args.gemma);
    }

    #[test]
    fn test_args_gemmad_flag() {
        let args = Args::try_parse_from(["eunice", "--gemmad", "hi"]).unwrap();
        assert!(args.gemmad);
        assert!(!args.no_gemmad);
    }

    #[test]
    fn test_args_no_gemmad_flag() {
        let args = Args::try_parse_from(["eunice", "--no-gemmad", "hi"]).unwrap();
        assert!(args.no_gemmad);
        assert!(!args.gemmad);
    }

    #[test]
    fn test_args_gemmad_default_false() {
        let args = Args::try_parse_from(["eunice", "hi"]).unwrap();
        assert!(!args.gemmad);
        assert!(!args.no_gemmad);
    }

    #[test]
    fn test_args_agents() {
        let args =
            Args::try_parse_from(["eunice", "--webapp", "--agents", "/tmp/agents.toml"]).unwrap();
        assert_eq!(args.agents, Some("/tmp/agents.toml".to_string()));
        assert!(args.webapp);
    }

    #[test]
    fn test_args_agents_default_none() {
        let args = Args::try_parse_from(["eunice", "hi"]).unwrap();
        assert_eq!(args.agents, None);
    }

    #[test]
    fn test_args_install() {
        let args = Args::try_parse_from(["eunice", "--install"]).unwrap();
        assert!(args.install);
        assert!(!args.uninstall_service);
        assert!(!args.uninstall);
    }

    #[test]
    fn test_args_install_default_false() {
        let args = Args::try_parse_from(["eunice", "hi"]).unwrap();
        assert!(!args.install);
    }

    #[test]
    fn test_args_uninstall_service() {
        let args = Args::try_parse_from(["eunice", "--uninstall-service"]).unwrap();
        assert!(args.uninstall_service);
        assert!(!args.uninstall);
        assert!(!args.install);
    }

    #[test]
    fn test_args_uninstall_service_default_false() {
        let args = Args::try_parse_from(["eunice", "hi"]).unwrap();
        assert!(!args.uninstall_service);
    }

    #[test]
    fn test_args_uninstall_is_distinct_from_uninstall_service() {
        let args = Args::try_parse_from(["eunice", "--uninstall"]).unwrap();
        assert!(args.uninstall);
        assert!(!args.uninstall_service);
    }

    #[test]
    fn test_args_install_composition() {
        let args = Args::try_parse_from([
            "eunice",
            "--install",
            "--port",
            "9000",
            "--host",
            "127.0.0.1",
            "--model",
            "sonnet",
            "--agents",
            "/tmp/agents.toml",
            "--no-persist",
        ])
        .unwrap();
        assert!(args.install);
        assert_eq!(args.port, 9000);
        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.model, Some("sonnet".to_string()));
        assert_eq!(args.agents, Some("/tmp/agents.toml".to_string()));
        assert!(args.no_persist);
    }
}
