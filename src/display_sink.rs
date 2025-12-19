//! Display output abstraction for coordinated terminal output.
//!
//! This module provides a trait-based abstraction for display output, allowing
//! the TUI mode to use SharedWriter while normal mode uses println!().

use colored::*;
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
    /// Response content from LLM
    Response { content: String },
    /// Error message
    Error { message: String },
    /// Debug message (only shown in verbose mode)
    Debug { message: String },
    /// Newline (for future use)
    #[allow(dead_code)]
    Newline,
}

/// Trait for display output sinks
pub trait DisplaySink: Send + Sync {
    /// Write a display event
    fn write_event(&self, event: DisplayEvent);

    /// Check if verbose mode is enabled
    fn is_verbose(&self) -> bool;
}

/// Standard output sink using println!()
pub struct StdDisplaySink {
    verbose: bool,
}

impl StdDisplaySink {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl DisplaySink for StdDisplaySink {
    fn write_event(&self, event: DisplayEvent) {
        match event {
            DisplayEvent::ThinkingStart => {
                println!("  {} Thinking...", "⋯".yellow());
            }
            DisplayEvent::ThinkingStop => {
                // No-op for standard output
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
                        "{}\n    ...{} more lines",
                        truncated.join("\n    "),
                        remaining
                    )
                } else {
                    result
                        .lines()
                        .map(|l| format!("    {}", l))
                        .collect::<Vec<_>>()
                        .join("\n")
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
            DisplayEvent::Error { message } => {
                eprintln!("{} {}", "❌".red(), message.red());
            }
            DisplayEvent::Debug { message } => {
                if self.verbose {
                    eprintln!("  {}", message.dimmed());
                }
            }
            DisplayEvent::Newline => {
                println!();
            }
        }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }
}

/// Silent sink that discards all output
pub struct SilentDisplaySink;

impl DisplaySink for SilentDisplaySink {
    fn write_event(&self, event: DisplayEvent) {
        // Only show responses, discard everything else
        if let DisplayEvent::Response { content } = event {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                println!("{}", trimmed);
            }
        }
    }

    fn is_verbose(&self) -> bool {
        false
    }
}

/// TUI display sink using SharedWriter for coordinated output
pub struct TuiDisplaySink {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    verbose: bool,
}

impl TuiDisplaySink {
    pub fn new<W: Write + Send + 'static>(writer: W, verbose: bool) -> Self {
        Self {
            writer: Arc::new(Mutex::new(Box::new(writer))),
            verbose,
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
                        "{}\n    ...{} more lines",
                        truncated.join("\n    "),
                        remaining
                    )
                } else {
                    result
                        .lines()
                        .map(|l| format!("    {}", l))
                        .collect::<Vec<_>>()
                        .join("\n")
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
            DisplayEvent::Error { message } => {
                let _ = writeln!(writer, "{RED}❌ {}{RESET}", message);
            }
            DisplayEvent::Debug { message } => {
                if self.verbose {
                    let _ = writeln!(writer, "  {DIM}{}{RESET}", message);
                }
            }
            DisplayEvent::Newline => {
                let _ = writeln!(writer);
            }
        }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }
}

/// Create a display sink based on mode
pub fn create_display_sink(silent: bool, verbose: bool) -> Arc<dyn DisplaySink> {
    if silent {
        Arc::new(SilentDisplaySink)
    } else {
        Arc::new(StdDisplaySink::new(verbose))
    }
}
