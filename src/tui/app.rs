//! TUI application using r3bl_tui's readline_async for proper output coordination.

use crate::agent::{self, AgentStatus};
use crate::client::Client;
use crate::compact::CompactionConfig;
use crate::display_sink::TuiDisplaySink;
use crate::models::Message;
use crate::models::ProviderInfo;
use crate::output_store::OutputStore;
use crate::theme;
use crate::tools::ToolRegistry;
use crate::usage::SessionUsage;

use anyhow::{anyhow, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{Clear, ClearType},
};
use r3bl_tui::{
    choose, height, DefaultIoDevices, HowToChoose, ReadlineAsyncContext, ReadlineEvent,
    SharedWriter, StyleSheet,
};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// ANSI color codes
const PURPLE: &str = "\x1b[38;5;141m";  // Light purple
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// Print header with ASCII art
fn print_header(shared_writer: &mut SharedWriter) -> std::io::Result<()> {
    writeln!(shared_writer)?;
    writeln!(shared_writer, "{PURPLE}  ┌─────────────┐{RESET}")?;
    writeln!(shared_writer, "{PURPLE}  │{RESET}   {PURPLE}EUNICE{RESET}    {PURPLE}│{RESET} {DIM}v{}{RESET}", env!("CARGO_PKG_VERSION"))?;
    writeln!(shared_writer, "{PURPLE}  └─────────────┘{RESET}")?;
    writeln!(shared_writer)?;
    Ok(())
}

/// Print status bar with model and tool info
fn print_status(
    shared_writer: &mut SharedWriter,
    model: &str,
    tool_count: usize,
) -> std::io::Result<()> {
    write!(shared_writer, "  {DIM}")?;
    write!(shared_writer, "{PURPLE}model:{RESET}{DIM} {model}")?;
    write!(shared_writer, "  {PURPLE}tools:{RESET}{DIM} {tool_count}")?;
    writeln!(shared_writer, "{RESET}\n")?;
    Ok(())
}

/// Available commands
const COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show this help"),
    ("/clear", "Clear conversation history"),
    ("/status", "Show current status"),
    ("/quit", "Exit TUI mode"),
];

/// Print help text
fn print_help(shared_writer: &mut SharedWriter) -> std::io::Result<()> {
    writeln!(shared_writer, "\n{DIM}Commands:{RESET}")?;
    for (cmd, desc) in COMMANDS {
        writeln!(shared_writer, "  {PURPLE}{cmd}{RESET}      {desc}")?;
    }
    writeln!(
        shared_writer,
        "\n{DIM}Type / for command menu, Ctrl+D to exit, Esc/Ctrl+C (twice in 500ms) to stop generation{RESET}\n"
    )?;
    Ok(())
}

/// Show command selection menu
async fn show_command_menu(ctx: &ReadlineAsyncContext) -> Result<Option<String>> {
    let items: Vec<String> = COMMANDS.iter().map(|(cmd, desc)| format!("{cmd}  {DIM}{desc}{RESET}")).collect();

    let mut io = DefaultIoDevices::default();
    io.maybe_shared_writer = Some(ctx.clone_shared_writer());
    let (output, input, shared) = io.as_mut_tuple();

    let result = choose(
        "Select command:",
        items,
        Some(height(6)),  // max height
        None,             // max width (auto)
        HowToChoose::Single,
        StyleSheet::default(),
        (output, input, shared),
    )
    .await
    .map_err(|e| anyhow!("Menu error: {}", e))?;

    // Extract the command from the selection (first word)
    if let Some(selected) = result.first() {
        let cmd = selected.split_whitespace().next().unwrap_or("");
        Ok(Some(cmd.to_string()))
    } else {
        Ok(None)
    }
}

use super::frame_editor::{self, LineResult};

/// Entry point for `--chat` / `--tui`. Defaults to the Claude-style framed editor;
/// set `EUNICE_TUI_CLASSIC=1` to use the legacy r3bl readline path.
pub async fn run_tui_mode(
    client: &Client,
    provider_info: &ProviderInfo,
    initial_prompt: Option<&str>,
) -> Result<()> {
    if std::env::var("EUNICE_TUI_CLASSIC").is_ok() {
        return run_tui_classic(client, provider_info, initial_prompt).await;
    }
    run_tui_framed(client, provider_info, initial_prompt).await
}

/// A stdout writer that translates `\n` -> `\r\n` so agent output renders correctly while the
/// terminal is in raw mode (raw mode lets us poll Esc-to-cancel during generation).
struct RawStdoutWriter;
impl std::io::Write for RawStdoutWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut out = std::io::stdout().lock();
        let mut last = 0;
        for (i, &b) in buf.iter().enumerate() {
            if b == b'\n' {
                out.write_all(&buf[last..i])?;
                out.write_all(b"\r\n")?;
                last = i + 1;
            }
        }
        out.write_all(&buf[last..])?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

/// Print raw-safely while raw mode is enabled.
fn raw_print(s: &str) {
    use std::io::Write as _;
    let mut w = RawStdoutWriter;
    let _ = w.write_all(s.as_bytes());
    let _ = w.flush();
}

/// Poll for Esc / Ctrl+C during generation and signal cancellation. Does NOT toggle raw mode
/// (the framed session keeps raw mode enabled throughout).
async fn monitor_cancel_raw(cancel_tx: watch::Sender<bool>) {
    loop {
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                if key_event.code == KeyCode::Esc
                    || (key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL))
                {
                    let _ = cancel_tx.send(true);
                    return;
                }
            }
        }
        tokio::task::yield_now().await;
    }
}

/// Run one prompt through the agent with a raw-mode-safe sink + cancel monitor.
#[allow(clippy::too_many_arguments)]
async fn run_generation(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_registry: &ToolRegistry,
    conversation_history: &mut Vec<Message>,
    output_store: &mut OutputStore,
    session_usage: &mut SessionUsage,
) {
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let cancel_handle = tokio::spawn(async move { monitor_cancel_raw(cancel_tx).await; });

    let display: Arc<dyn crate::display_sink::DisplaySink> =
        Arc::new(TuiDisplaySink::new(RawStdoutWriter));

    let result = agent::run_agent_cancellable(
        client,
        model,
        prompt,
        50,
        tool_registry,
        display,
        conversation_history,
        Some(cancel_rx),
        Some(CompactionConfig::default()),
        Some(output_store),
    )
    .await;

    cancel_handle.abort();
    // Wait for the monitor to actually stop so it can't race the editor's stdin reads next turn.
    let _ = cancel_handle.await;

    match result {
        Ok(r) => {
            session_usage.total_input_tokens += r.usage.total_input_tokens;
            session_usage.total_output_tokens += r.usage.total_output_tokens;
            session_usage.total_cached_tokens += r.usage.total_cached_tokens;
            session_usage.api_calls += r.usage.api_calls;
            if r.status == AgentStatus::Cancelled {
                raw_print(&format!("\r\n{YELLOW}⚠ Stopped by user{RESET}\r\n"));
            } else {
                raw_print("\r\n");
            }
        }
        Err(e) => {
            raw_print(&format!("\r\n{YELLOW}Error: {e}{RESET}\r\n"));
        }
    }
}

/// Claude-style framed `--chat` loop (the default).
async fn run_tui_framed(
    client: &Client,
    provider_info: &ProviderInfo,
    initial_prompt: Option<&str>,
) -> Result<()> {
    let model = provider_info.resolved_model.clone();
    let tool_registry = ToolRegistry::new();
    let tool_count = tool_registry.get_tools().len();
    let mut conversation_history: Vec<Message> = Vec::new();
    let mut input_history: Vec<String> = Vec::new();
    let mut output_store = OutputStore::new();
    let mut session_usage = SessionUsage::new();

    // Header (printed in cooked mode, before raw mode is enabled).
    println!();
    println!("{PURPLE}  eunice{RESET}  {DIM}v{}{RESET}", env!("CARGO_PKG_VERSION"));
    println!("{DIM}  model: {model}  ·  tools: {tool_count}{RESET}");
    println!("{DIM}  /help for commands · /quit or Ctrl+D to exit{RESET}");

    let footer = "↵ send · esc clear · /help · ctrl+d exit";

    crossterm::terminal::enable_raw_mode()
        .map_err(|e| anyhow!("Failed to enable raw mode: {}", e))?;

    if let Some(p) = initial_prompt {
        raw_print(&format!("\r\n{}\r\n", theme::user_bar(p)));
        run_generation(
            client, &model, p, &tool_registry,
            &mut conversation_history, &mut output_store, &mut session_usage,
        )
        .await;
        input_history.push(p.to_string());
    }

    loop {
        let line = match frame_editor::read_line_framed(&input_history, "eunice", footer) {
            Ok(LineResult::Line(s)) => s,
            Ok(LineResult::Eof) | Ok(LineResult::Interrupted) => break,
            Err(e) => {
                let _ = crossterm::terminal::disable_raw_mode();
                return Err(anyhow!("input error: {}", e));
            }
        };
        let input = line.trim().to_string();
        if input.is_empty() {
            continue;
        }
        let lower = input.to_lowercase();
        if lower == "exit" || lower == "quit" || lower == "/quit" || lower == "/q" || lower == "/exit" {
            break;
        }
        match input.as_str() {
            "/help" | "/h" | "/?" => {
                raw_print(&format!("\r\n{DIM}Commands:{RESET}\r\n  /help  /clear  /status  /quit\r\n"));
                continue;
            }
            "/clear" | "/c" => {
                conversation_history.clear();
                raw_print(&format!("\r\n{GREEN}Conversation history cleared.{RESET}\r\n"));
                continue;
            }
            "/status" | "/s" => {
                raw_print(&format!(
                    "\r\n{DIM}  model: {model} · tools: {tool_count} · history: {} msgs{RESET}\r\n",
                    conversation_history.len()
                ));
                continue;
            }
            _ => {}
        }

        raw_print(&format!("\r\n{}\r\n", theme::user_bar(&input)));
        if !input.starts_with('/') && input_history.last() != Some(&input) {
            input_history.push(input.clone());
        }
        run_generation(
            client, &model, &input, &tool_registry,
            &mut conversation_history, &mut output_store, &mut session_usage,
        )
        .await;
    }

    let _ = crossterm::terminal::disable_raw_mode();

    if session_usage.has_usage() {
        println!();
        let summary = session_usage.format_summary(&model, &provider_info.provider);
        for l in summary.lines() {
            println!("{DIM}{l}{RESET}");
        }
    }
    println!("\n{DIM}Goodbye!{RESET}\n");
    Ok(())
}

/// Legacy r3bl_tui readline path (set `EUNICE_TUI_CLASSIC=1` to use).
async fn run_tui_classic(
    client: &Client,
    provider_info: &ProviderInfo,
    initial_prompt: Option<&str>,
) -> Result<()> {
    // Create readline context with custom prompt
    let prompt = format!("{PURPLE}›{RESET} ");
    let maybe_ctx = ReadlineAsyncContext::try_new(Some(prompt), None)
        .await
        .map_err(|e| anyhow!("Failed to create readline context: {}", e))?;

    let Some(mut ctx) = maybe_ctx else {
        eprintln!("Terminal is not interactive. Falling back to standard mode.");
        return crate::interactive::interactive_mode(
            client,
            &provider_info.resolved_model,
            initial_prompt,
        )
        .await;
    };

    let mut shared_writer = ctx.clone_shared_writer();

    // Suppress r3bl's echo of the submitted line — we render user turns as an inverse bar.
    ctx.readline.should_print_line_on(false, false);

    // Print header and status
    print_header(&mut shared_writer)?;

    // Create tool registry and count tools
    let tool_registry = ToolRegistry::new();
    let tool_count = tool_registry.get_tools().len();

    print_status(&mut shared_writer, &provider_info.resolved_model, tool_count)?;
    print_help(&mut shared_writer)?;

    // Conversation history
    let mut conversation_history: Vec<Message> = Vec::new();

    // Output store for truncating large tool outputs
    let mut output_store = OutputStore::new();

    // Session-level token usage tracking
    let mut session_usage = SessionUsage::new();

    // Process initial prompt if provided
    if let Some(prompt_text) = initial_prompt {
        // Show the user's prompt as an inverse bar
        writeln!(shared_writer, "{}", theme::user_bar(prompt_text))?;
        writeln!(shared_writer)?;
        process_prompt(
            &mut ctx,
            client,
            provider_info,
            prompt_text,
            &tool_registry,
            &mut conversation_history,
            &mut output_store,
            &mut session_usage,
        )
        .await?;
    }

    // Main event loop - use r3bl_tui's native readline
    loop {
        // Draw the input-box top rule with a right-aligned mode label just above the prompt.
        {
            let mut sw = ctx.clone_shared_writer();
            let _ = writeln!(sw, "{}", theme::rule(Some("eunice")));
        }

        let event = ctx.readline.readline().await;

        match event {
            Ok(ReadlineEvent::Line(line)) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                // Handle exit/quit without slash (case insensitive)
                let input_lower = input.to_lowercase();
                if input_lower == "exit" || input_lower == "quit" {
                    let mut sw = ctx.clone_shared_writer();
                    if session_usage.has_usage() {
                        writeln!(sw)?;
                        let summary = session_usage.format_summary(&provider_info.resolved_model, &provider_info.provider);
                        for line in summary.lines() {
                            writeln!(sw, "{DIM}{}{RESET}", line)?;
                        }
                    }
                    writeln!(sw, "\n{DIM}Goodbye!{RESET}\n")?;
                    break;
                }

                // Handle commands
                if input.starts_with('/') {
                    // If just "/" show command menu
                    let cmd = if input == "/" {
                        match show_command_menu(&ctx).await? {
                            Some(c) => c,
                            None => continue,  // User cancelled
                        }
                    } else {
                        input.to_string()
                    };

                    let mut sw = ctx.clone_shared_writer();
                    match cmd.as_str() {
                        "/help" | "/h" | "/?" => {
                            print_help(&mut sw)?;
                        }
                        "/clear" | "/c" => {
                            conversation_history.clear();
                            writeln!(sw, "\n{GREEN}Conversation history cleared.{RESET}\n")?;
                        }
                        "/status" | "/s" => {
                            writeln!(sw)?;
                            print_status(
                                &mut sw,
                                &provider_info.resolved_model,
                                tool_count,
                            )?;
                            writeln!(
                                sw,
                                "  {DIM}History: {} messages{RESET}",
                                conversation_history.len()
                            )?;
                            // Display token usage if any
                            if session_usage.has_usage() {
                                let summary = session_usage.format_summary(&provider_info.resolved_model, &provider_info.provider);
                                for line in summary.lines() {
                                    writeln!(sw, "  {DIM}{}{RESET}", line)?;
                                }
                            }
                            writeln!(sw)?;
                        }
                        "/quit" | "/q" | "/exit" => {
                            // Display session usage summary if any
                            if session_usage.has_usage() {
                                writeln!(sw)?;
                                let summary = session_usage.format_summary(&provider_info.resolved_model, &provider_info.provider);
                                for line in summary.lines() {
                                    writeln!(sw, "{DIM}{}{RESET}", line)?;
                                }
                            }
                            writeln!(sw, "\n{DIM}Goodbye!{RESET}\n")?;
                            break;
                        }
                        _ => {
                            writeln!(sw, "\n{YELLOW}Unknown command: {}{RESET}", cmd)?;
                            writeln!(sw, "{DIM}Type / for command menu{RESET}\n")?;
                        }
                    }
                    continue;
                }

                // Render the submitted turn as an inverse bar (r3bl's echo is suppressed).
                {
                    let mut sw = ctx.clone_shared_writer();
                    let _ = writeln!(sw, "{}", theme::user_bar(input));
                }

                // Add to history
                ctx.readline.add_history_entry(input.to_string());

                // Process the prompt
                process_prompt(
                    &mut ctx,
                    client,
                    provider_info,
                    input,
                    &tool_registry,
                    &mut conversation_history,
                    &mut output_store,
                    &mut session_usage,
                )
                .await?;
            }
            Ok(ReadlineEvent::Eof) | Ok(ReadlineEvent::Interrupted) => {
                let mut sw = ctx.clone_shared_writer();
                // Display session usage summary if any
                if session_usage.has_usage() {
                    writeln!(sw)?;
                    let summary = session_usage.format_summary(&provider_info.resolved_model, &provider_info.provider);
                    for line in summary.lines() {
                        writeln!(sw, "{DIM}{}{RESET}", line)?;
                    }
                }
                writeln!(sw, "\n{DIM}Goodbye!{RESET}\n")?;
                break;
            }
            Ok(ReadlineEvent::Resized(_)) => {
                // Handle terminal resize - continue loop
                continue;
            }
            Ok(_) => {
                // Handle other events (Tab, BackTab, PageUp, etc.) - ignore
                continue;
            }
            Err(e) => {
                // Handle readline errors
                let mut sw = ctx.clone_shared_writer();
                writeln!(sw, "\n{YELLOW}Readline error: {}{RESET}\n", e)?;
                break;
            }
        }
    }

    // Clear any prompt that r3bl_tui might print during cleanup
    execute!(std::io::stdout(), cursor::MoveToColumn(0), Clear(ClearType::CurrentLine))?;

    Ok(())
}

/// Monitor for Escape or Ctrl+C to cancel the running agent
async fn monitor_cancel_keys(cancel_tx: watch::Sender<bool>) {
    // Keep track of the last Ctrl+C press time for double-press detection
    let mut last_ctrlc_time: Option<Instant> = None;

    loop {
        // Poll for events with a small timeout
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                // Check for Escape key
                if key_event.code == KeyCode::Esc {
                    let _ = cancel_tx.send(true);
                    return;
                }

                // Check for Ctrl+C
                if key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    let now = Instant::now();
                    if let Some(prev_time) = last_ctrlc_time {
                        if now.duration_since(prev_time) < Duration::from_millis(500) {
                            // Double Ctrl+C within 500ms
                            let _ = cancel_tx.send(true);
                            return;
                        }
                    }
                    last_ctrlc_time = Some(now);
                }
            }
        }

        // Yield to other tasks
        tokio::task::yield_now().await;
    }
}

/// Process a single prompt
#[allow(clippy::too_many_arguments)]
async fn process_prompt(
    ctx: &mut ReadlineAsyncContext,
    client: &Client,
    provider_info: &ProviderInfo,
    prompt: &str,
    tool_registry: &ToolRegistry,
    conversation_history: &mut Vec<Message>,
    output_store: &mut OutputStore,
    session_usage: &mut SessionUsage,
) -> Result<()> {
    let mut shared_writer = ctx.clone_shared_writer();
    writeln!(shared_writer)?;

    // Create TuiDisplaySink using the SharedWriter for coordinated output
    let display: Arc<dyn crate::display_sink::DisplaySink> = Arc::new(
        TuiDisplaySink::new(ctx.clone_shared_writer())
    );

    // Create cancellation channel
    let (cancel_tx, cancel_rx) = watch::channel(false);

    // Spawn cancel key monitor
    let cancel_handle = tokio::spawn(async move {
        monitor_cancel_keys(cancel_tx).await;
    });

    // Run the agent with the TuiDisplaySink
    let result = agent::run_agent_cancellable(
        client,
        &provider_info.resolved_model,
        prompt,
        50, // tool_output_limit
        tool_registry,
        display,
        conversation_history,
        Some(cancel_rx),
        Some(CompactionConfig::default()),
        Some(output_store),
    )
    .await
    .map(|r| {
        // Accumulate usage from this run
        session_usage.total_input_tokens += r.usage.total_input_tokens;
        session_usage.total_output_tokens += r.usage.total_output_tokens;
        session_usage.total_cached_tokens += r.usage.total_cached_tokens;
        session_usage.api_calls += r.usage.api_calls;
        if r.status == AgentStatus::Cancelled { Some(true) } else { None }
    });

    // Stop cancel key monitoring
    cancel_handle.abort();

    // Handle result
    let mut sw = ctx.clone_shared_writer();
    match result {
        Ok(Some(true)) => {
            // Cancelled
            writeln!(sw, "\n{YELLOW}⚠ Stopped by user{RESET}\n")?;
        }
        Err(e) => {
            writeln!(sw, "\n{YELLOW}Error: {}{RESET}\n", e)?;
        }
        _ => {
            writeln!(sw)?;
        }
    }

    Ok(())
}
