//! Display output abstraction for coordinated terminal output.
//!
//! This module provides a trait-based abstraction for display output, allowing
//! the TUI mode to use SharedWriter while normal mode uses println!().

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Events that can be displayed
#[derive(Debug, Clone)]
pub enum DisplayEvent {
    /// Thinking indicator started
    ThinkingStart,
    /// Thinking indicator stopped
    ThinkingStop,
    /// Tool call being made
    ToolCall { name: String, arguments: String },
    /// Tool result received
    ToolResult { result: String, limit: usize },
    /// Response content from LLM (complete)
    Response { content: String },
    /// Streaming chunk from LLM (partial, no newline)
    StreamChunk { content: String },
    /// Streaming complete (print newline)
    StreamEnd,
    /// Error message
    Error { message: String },
}

/// Trait for display output sinks
pub trait DisplaySink: Send + Sync {
    /// Write a display event
    fn write_event(&self, event: DisplayEvent);
}

/// Standard output sink using println!() with animated spinner
pub struct StdDisplaySink {
    spinner: Mutex<Option<ProgressBar>>,
}

impl StdDisplaySink {
    pub fn new() -> Self {
        Self {
            spinner: Mutex::new(None),
        }
    }

    fn start_spinner(&self) {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("  {spinner:.magenta} {msg:.magenta}")
                .unwrap(),
        );
        spinner.set_message("Thinking...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        *self.spinner.lock().unwrap() = Some(spinner);
    }

    fn stop_spinner(&self) {
        if let Some(spinner) = self.spinner.lock().unwrap().take() {
            spinner.finish_and_clear();
        }
    }
}

impl Default for StdDisplaySink {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplaySink for StdDisplaySink {
    fn write_event(&self, event: DisplayEvent) {
        match event {
            DisplayEvent::ThinkingStart => {
                self.start_spinner();
            }
            DisplayEvent::ThinkingStop => {
                self.stop_spinner();
            }
            DisplayEvent::ToolCall { name, arguments } => {
                println!("  {} {}", "→".blue(), name.bright_blue());
                // Show arguments in grey if not empty
                if !arguments.is_empty() {
                    // Pretty-print JSON if possible, otherwise show as-is
                    let display_args = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&arguments) {
                        serde_json::to_string_pretty(&parsed).unwrap_or(arguments)
                    } else {
                        arguments
                    };
                    for line in display_args.lines() {
                        println!("    {}", line.dimmed());
                    }
                }
            }
            DisplayEvent::ToolResult { result, limit } => {
                let lines: Vec<&str> = result.lines().collect();
                let output = if limit > 0 && lines.len() > limit {
                    let truncated: Vec<&str> = lines.iter().take(limit).copied().collect();
                    let remaining = lines.len() - limit;
                    format!(
                        "{}\\n    ...{} more lines",
                        truncated.join("\\n    "),
                        remaining
                    )
                } else {
                    result
                        .lines()
                        .map(|l| format!("    {}", l))
                        .collect::<Vec<_>>()
                        .join("\\n")
                };

                if !output.trim().is_empty() {
                    println!("{}", output.dimmed());
                }
            }
            DisplayEvent::Response { content } => {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    println!("{}", trimmed);
                }
            }
            DisplayEvent::StreamChunk { content } => {
                print!("{}", content);
                let _ = std::io::stdout().flush();
            }
            DisplayEvent::StreamEnd => {
                println!();
            }
            DisplayEvent::Error { message } => {
                eprintln!("{} {}", "❌".red(), message.red());
            }
        }
    }
}

/// TUI display sink using SharedWriter for coordinated output
pub struct TuiDisplaySink {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl TuiDisplaySink {
    pub fn new<W: Write + Send + 'static>(writer: W) -> Self {
        Self {
            writer: Arc::new(Mutex::new(Box::new(writer))),
        }
    }
}

impl DisplaySink for TuiDisplaySink {
    fn write_event(&self, event: DisplayEvent) {
        let mut writer = self.writer.lock().unwrap();

        // ANSI codes
        const YELLOW: &str = "\x1b[33m";
        const BLUE: &str = "\x1b[34m";
        const BRIGHT_BLUE: &str = "\x1b[94m";
        const RED: &str = "\x1b[31m";
        const DIM: &str = "\x1b[2m";
        const RESET: &str = "\x1b[0m";

        match event {
            DisplayEvent::ThinkingStart => {
                let _ = writeln!(writer, "  {YELLOW}⋯{RESET} Thinking...");
            }
            DisplayEvent::ThinkingStop => {
                // No-op
            }
            DisplayEvent::ToolCall { name, arguments } => {
                let _ = writeln!(writer, "  {BLUE}→{RESET} {BRIGHT_BLUE}{}{RESET}", name);
                // Show arguments in grey if not empty
                if !arguments.is_empty() {
                    // Pretty-print JSON if possible, otherwise show as-is
                    let display_args = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&arguments) {
                        serde_json::to_string_pretty(&parsed).unwrap_or(arguments)
                    } else {
                        arguments
                    };
                    for line in display_args.lines() {
                        let _ = writeln!(writer, "    {DIM}{}{RESET}", line);
                    }
                }
            }
            DisplayEvent::ToolResult { result, limit } => {
                let lines: Vec<&str> = result.lines().collect();
                let output = if limit > 0 && lines.len() > limit {
                    let truncated: Vec<&str> = lines.iter().take(limit).copied().collect();
                    let remaining = lines.len() - limit;
                    format!(
                        "{}\\n    ...{} more lines",
                        truncated.join("\\n    "),
                        remaining
                    )
                } else {
                    result
                        .lines()
                        .map(|l| format!("    {}", l))
                        .collect::<Vec<_>>()
                        .join("\\n")
                };

                if !output.trim().is_empty() {
                    let _ = writeln!(writer, "{DIM}{}{RESET}", output);
                }
            }
            DisplayEvent::Response { content } => {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    let _ = writeln!(writer, "{}", trimmed);
                }
            }
            DisplayEvent::StreamChunk { content } => {
                let _ = write!(writer, "{}", content);
                let _ = writer.flush();
            }
            DisplayEvent::StreamEnd => {
                let _ = writeln!(writer);
            }
            DisplayEvent::Error { message } => {
                let _ = writeln!(writer, "{RED}❌ {}{RESET}", message);
            }
        }
    }
}

/// Create a display sink for standard output
pub fn create_display_sink() -> Arc<dyn DisplaySink> {
    Arc::new(StdDisplaySink::new())
}
