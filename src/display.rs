use crate::models::Provider;
use crate::provider::get_available_models;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Spinner handle that can be used to stop the spinner
pub struct Spinner(ProgressBar);

impl Spinner {
    /// Start a new spinner with a message
    pub fn start(message: &str) -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        Self(pb)
    }

    /// Update the spinner message
    pub fn set_message(&self, message: &str) {
        self.0.set_message(message.to_string());
    }

    /// Stop the spinner
    pub async fn stop(self) {
        self.0.finish_and_clear();
    }
}

/// Thinking spinner that shows elapsed time
pub struct ThinkingSpinner {
    pb: ProgressBar,
    running: Arc<AtomicBool>,
}

impl ThinkingSpinner {
    /// Start a new thinking spinner with elapsed time counter
    pub fn start() -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.yellow} {msg}")
                .unwrap(),
        );
        pb.set_message("Thinking... 0s");

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let pb_clone = pb.clone();

        // Spawn a task to update the elapsed time every second
        tokio::spawn(async move {
            let mut seconds = 0u64;
            while running_clone.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if running_clone.load(Ordering::Relaxed) {
                    seconds += 1;
                    pb_clone.set_message(format!("Thinking... {}s", seconds));
                }
            }
        });

        pb.enable_steady_tick(Duration::from_millis(80));
        Self { pb, running }
    }

    /// Stop the thinking spinner
    pub fn stop(self) {
        self.running.store(false, Ordering::Relaxed);
        self.pb.finish_and_clear();
    }
}

/// Print a tool call
pub fn print_tool_call(tool_name: &str, arguments: &str) {
    println!(
        "{} {}({})",
        "🔧".blue(),
        tool_name.bright_blue(),
        arguments.dimmed()
    );
}

/// Print a tool result
pub fn print_tool_result(result: &str, limit: usize) {
    let lines: Vec<&str> = result.lines().collect();
    let output = if limit > 0 && lines.len() > limit {
        let truncated: Vec<&str> = lines.iter().take(limit).cloned().collect();
        let remaining = lines.len() - limit;
        format!("{}\n{}", truncated.join("\n"), format!("...{} more lines", remaining).dimmed())
    } else {
        result.to_string()
    };

    println!("{} {}", "→".green(), output.green());
}

/// Print model information
pub fn print_model_info(model: &str, provider: &Provider) {
    println!(
        "{} {} ({})",
        provider.get_icon(),
        model.yellow().bold(),
        provider.to_string().yellow()
    );
}

/// Print MCP server information (simplified - just server names and tool counts)
pub fn print_mcp_info(servers: &[(String, usize, Vec<String>)]) {
    if servers.is_empty() {
        return;
    }

    let total_tools: usize = servers.iter().map(|(_, count, _)| count).sum();
    let server_names: Vec<&str> = servers.iter().map(|(name, _, _)| name.as_str()).collect();

    println!(
        "{} {} ({} tools from {})",
        "🔌",
        "MCP".yellow().bold(),
        total_tools.to_string().yellow(),
        server_names.join(", ").dimmed()
    );
}

/// Print agent startup information
pub fn print_agent_info(agent_name: &str, tools_count: usize, subagents: &[String]) {
    let subagents_str = if subagents.is_empty() {
        "no subagents".dimmed().to_string()
    } else {
        format!("{} subagents", subagents.len()).to_string()
    };

    println!(
        "{} Agent {} ({} tools, {})",
        "🤖",
        agent_name.yellow().bold(),
        tools_count.to_string().yellow(),
        subagents_str.dimmed()
    );
}

/// Print agent invocation (when one agent calls another)
pub fn print_agent_invoke(from_agent: &str, to_agent: &str, task: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let task_preview = if task.len() > 60 {
        format!("{}...", &task[..57])
    } else {
        task.to_string()
    };

    println!(
        "{}{}→{} {}",
        indent,
        from_agent.blue(),
        to_agent.yellow().bold(),
        task_preview.dimmed()
    );
}

/// Print agent completion
pub fn print_agent_complete(agent_name: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    println!(
        "{}{}←{} {}",
        indent,
        "".green(),
        agent_name.yellow(),
        "done".dimmed()
    );
}

/// Print DMN mode indicator
pub fn print_dmn_mode() {
    println!("{} {}", "🧠", "DMN Mode".yellow().bold());
}

/// Print error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", "❌".red(), message.red());
}

/// Print user stopped message (when user cancels with Escape)
pub fn print_user_stopped() {
    eprintln!("{}", "⏹ User Stopped".yellow().bold());
}

/// Print verbose debug message
pub fn debug(message: &str, verbose: bool) {
    if verbose {
        eprintln!("{}", message.dimmed());
    }
}

/// Print available models in a list format
pub fn print_model_list() {
    println!("\n{}\n", "Available Models".bold().underline());

    let models = get_available_models();

    for (provider, model_list, available) in models {
        let icon = provider.get_icon();
        let status = if available { "✅" } else { "❌" };
        let key_status = get_key_status(&provider, available);

        println!(
            "{} {} {} ({})",
            icon,
            provider.to_string().bold(),
            status,
            key_status.dimmed()
        );

        if model_list.is_empty() {
            println!("   {}", "No models available".dimmed());
        } else {
            for model in &model_list {
                println!("   {} {}", "•".dimmed(), model);
            }
        }
        println!();
    }
}

/// Get the status of the API key for a given provider
fn get_key_status(provider: &Provider, available: bool) -> String {
    match provider {
        Provider::Ollama => {
            if available {
                "running".to_string()
            } else {
                "not running".to_string()
            }
        }
        _ => {
            let key_name = match provider {
                Provider::OpenAI => "OPENAI_API_KEY",
                Provider::Gemini => "GEMINI_API_KEY",
                Provider::Anthropic => "ANTHROPIC_API_KEY",
                _ => unreachable!(),
            };
            if let Ok(key) = env::var(key_name) {
                if key.len() >= 4 {
                    format!("...{}", &key[key.len() - 4..])
                } else {
                    "set".to_string()
                }
            } else {
                "not set".to_string()
            }
        }
    }
}
