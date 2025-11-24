use crate::models::Provider;
use crate::provider::get_available_models;
use colored::*;
use crossterm::{cursor, terminal, ExecutableCommand};
use std::env;
use std::io::{stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Spinner frames for progress indication
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Spinner handle that can be used to stop the spinner and display output
pub struct Spinner {
    stop_signal: Arc<AtomicBool>,
    output_tx: mpsc::UnboundedSender<String>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Spinner {
    /// Start a new spinner with a message
    pub fn start(message: &str) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();
        let message = message.to_string();
        let (output_tx, mut output_rx) = mpsc::unbounded_channel::<String>();

        let handle = tokio::spawn(async move {
            let mut frame_idx = 0;
            let mut stdout = stdout();
            let mut output_lines: Vec<String> = Vec::new();

            // Hide cursor for cleaner display
            let _ = stdout.execute(cursor::Hide);

            loop {
                // Check for new output
                while let Ok(line) = output_rx.try_recv() {
                    output_lines.push(line);
                }

                // Clear current line and display spinner
                let _ = stdout.execute(terminal::Clear(terminal::ClearType::CurrentLine));
                let _ = stdout.execute(cursor::MoveToColumn(0));

                let frame = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];

                // Show output preview if available
                let display_msg = if !output_lines.is_empty() {
                    let last_line = output_lines.last().unwrap();
                    let truncated = if last_line.len() > 60 {
                        format!("{}...", &last_line[..57])
                    } else {
                        last_line.clone()
                    };
                    format!("{} {} {} {}", frame.cyan(), message.cyan(), "→".dimmed(), truncated.dimmed())
                } else {
                    format!("{} {}", frame.cyan(), message.cyan())
                };

                print!("{}", display_msg);
                let _ = stdout.flush();

                if stop_signal_clone.load(Ordering::Relaxed) {
                    break;
                }

                frame_idx += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
            }

            // Clear the spinner line
            let _ = stdout.execute(terminal::Clear(terminal::ClearType::CurrentLine));
            let _ = stdout.execute(cursor::MoveToColumn(0));
            let _ = stdout.execute(cursor::Show);
            let _ = stdout.flush();
        });

        Spinner {
            stop_signal,
            output_tx,
            handle: Some(handle),
        }
    }

    /// Send output to display alongside the spinner
    pub fn add_output(&self, line: &str) {
        let _ = self.output_tx.send(line.to_string());
    }

    /// Stop the spinner
    pub async fn stop(mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
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

/// Print DMN mode indicator
pub fn print_dmn_mode() {
    println!("{} {}", "🧠", "DMN Mode".yellow().bold());
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
