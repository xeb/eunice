use crate::models::Provider;
use crate::provider::get_available_models;
use colored::*;
use std::env;

// Note: ThinkingSpinner, print_tool_call, and print_tool_result are now handled
// by DisplaySink trait in display_sink.rs. The functions below are kept for
// non-TUI modes that haven't been migrated yet.

/// Thinking indicator (no animation to avoid terminal conflicts)
#[allow(dead_code)]
pub struct ThinkingSpinner;

#[allow(dead_code)]
impl ThinkingSpinner {
    /// Start thinking indicator - just prints once
    pub fn start() -> Self {
        println!("  {} Thinking...", "⋯".yellow());
        Self
    }

    /// Stop thinking indicator - no-op since we already printed newline
    pub fn stop(self) {
        // Nothing to clean up
    }
}

/// Print a tool call
#[allow(dead_code)]
pub fn print_tool_call(tool_name: &str) {
    println!("  {} {}", "→".blue(), tool_name.bright_blue());
}

/// Print a tool result
#[allow(dead_code)]
pub fn print_tool_result(result: &str, limit: usize) {
    let lines: Vec<&str> = result.lines().collect();
    let output = if limit > 0 && lines.len() > limit {
        let truncated: Vec<&str> = lines.iter().take(limit).cloned().collect();
        let remaining = lines.len() - limit;
        format!("{}\n    ...{} more lines", truncated.join("\n    "), remaining)
    } else {
        result.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
    };

    if !output.trim().is_empty() {
        println!("{}", output.dimmed());
    }
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

/// Print Research mode indicator
pub fn print_research_mode() {
    println!("{} {}", "🔬", "Research Mode".cyan().bold());
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
        eprintln!("  {}", message.dimmed());
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
