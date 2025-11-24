use crate::models::Provider;
use crate::provider::get_available_models;
use colored::*;
use std::env;

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
    let icon = match provider {
        Provider::OpenAI => "🤖",
        Provider::Gemini => "💎",
        Provider::Anthropic => "🧠",
        Provider::Ollama => "🦙",
    };

    println!(
        "{} {} ({})",
        icon,
        model.yellow().bold(),
        provider.to_string().yellow()
    );
}

/// Print MCP server information
pub fn print_mcp_info(servers: &[(String, usize, Vec<String>)]) {
    if servers.is_empty() {
        return;
    }

    println!("{} {}", "🔌", "MCP Servers".yellow().bold());

    for (name, count, tools) in servers {
        println!("  {} {} ({} tools)", "📡".dimmed(), name.yellow(), count);

        // Show up to 4 tools
        for tool in tools.iter().take(4) {
            println!("     {} {}", "•".dimmed(), tool.dimmed());
        }

        if tools.len() > 4 {
            println!("     {} {}", "•".dimmed(), format!("...{} more", tools.len() - 4).dimmed());
        }
    }
}

/// Print sysadmin mode indicator
pub fn print_sysadmin_mode() {
    println!("{} {}", "🔧", "Sysadmin Mode".yellow().bold());
}

/// Print error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", "❌".red(), message.red());
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
        let icon = match provider {
            Provider::OpenAI => "🤖",
            Provider::Gemini => "💎",
            Provider::Anthropic => "🧠",
            Provider::Ollama => "🦙",
        };

        let status = if available { "✅" } else { "❌" };

        // Check for API key (show last 4 chars if available)
        let key_status = match provider {
            Provider::OpenAI => {
                if let Ok(key) = env::var("OPENAI_API_KEY") {
                    if key.len() >= 4 {
                        format!("...{}", &key[key.len() - 4..])
                    } else {
                        "set".to_string()
                    }
                } else {
                    "not set".to_string()
                }
            }
            Provider::Gemini => {
                if let Ok(key) = env::var("GEMINI_API_KEY") {
                    if key.len() >= 4 {
                        format!("...{}", &key[key.len() - 4..])
                    } else {
                        "set".to_string()
                    }
                } else {
                    "not set".to_string()
                }
            }
            Provider::Anthropic => {
                if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
                    if key.len() >= 4 {
                        format!("...{}", &key[key.len() - 4..])
                    } else {
                        "set".to_string()
                    }
                } else {
                    "not set".to_string()
                }
            }
            Provider::Ollama => {
                if available {
                    "running".to_string()
                } else {
                    "not running".to_string()
                }
            }
        };

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
