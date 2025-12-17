use crate::agent::{run_agent_cancellable, AgentResult};
use crate::client::Client;
use crate::compact::CompactionConfig;
use crate::config::DMN_INSTRUCTIONS;
use crate::display;
use crate::mcp::McpManager;
use crate::models::Message;
use crate::orchestrator::AgentOrchestrator;
use anyhow::Result;
use colored::Colorize;
use crossterm::cursor::{MoveTo, MoveToColumn, MoveUp, RestorePosition, SavePosition};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::ExecutableCommand;
use std::io::{self, Write};
use std::time::Duration;
use tokio::sync::watch;

/// Available slash commands for autocomplete
const SLASH_COMMANDS: &[&str] = &["/help", "/mcp"];

/// Get matching slash commands for autocomplete
fn get_matching_commands(prefix: &str) -> Vec<&'static str> {
    if prefix.is_empty() || !prefix.starts_with('/') {
        return vec![];
    }
    SLASH_COMMANDS
        .iter()
        .filter(|cmd| cmd.starts_with(prefix) && **cmd != prefix)
        .copied()
        .collect()
}

/// Show autocomplete suggestions above the current line
fn show_suggestions(stdout: &mut io::Stdout, suggestions: &[&str], showing_suggestions: &mut bool) -> io::Result<()> {
    if suggestions.is_empty() {
        if *showing_suggestions {
            clear_suggestions(stdout)?;
            *showing_suggestions = false;
        }
        return Ok(());
    }

    // Save cursor position
    stdout.execute(SavePosition)?;

    // Move up one line and clear it
    stdout.execute(MoveUp(1))?;
    stdout.execute(MoveToColumn(0))?;
    stdout.execute(Clear(ClearType::CurrentLine))?;

    // Print suggestions in dimmed grey
    let suggestion_text: Vec<String> = suggestions.iter().map(|s| s.to_string()).collect();
    print!("{}", suggestion_text.join("  ").dimmed());
    stdout.flush()?;

    // Restore cursor position
    stdout.execute(RestorePosition)?;

    *showing_suggestions = true;
    Ok(())
}

/// Clear the suggestions line
fn clear_suggestions(stdout: &mut io::Stdout) -> io::Result<()> {
    stdout.execute(SavePosition)?;
    stdout.execute(MoveUp(1))?;
    stdout.execute(MoveToColumn(0))?;
    stdout.execute(Clear(ClearType::CurrentLine))?;
    stdout.execute(RestorePosition)?;
    Ok(())
}

/// Print help message for interactive mode
fn print_help() {
    println!();
    println!("{}", "Interactive Mode Commands".bold().underline());
    println!();
    println!("  {}       Show this help message", "/help".cyan());
    println!("  {}        List configured MCP servers", "/mcp".cyan());
    println!("  {}       Exit interactive mode", "exit".cyan());
    println!("  {}       Exit interactive mode", "quit".cyan());
    println!();
    println!("{}", "Keyboard Shortcuts".bold().underline());
    println!();
    println!("  {}      Exit interactive mode", "Ctrl+C".yellow());
    println!("  {}      Exit (when line is empty)", "Ctrl+D".yellow());
    println!("  {}     Cancel current request while model is thinking", "Escape".yellow());
    println!();
    println!("  {}    Navigate command history", "Up/Down".yellow());
    println!("  {} Navigate within line", "Left/Right".yellow());
    println!("  {}   Jump to start/end of line", "Home/End".yellow());
    println!("  {}    Jump to start of line", "Ctrl+A".yellow());
    println!("  {}    Jump to end of line", "Ctrl+E".yellow());
    println!("  {}    Clear entire line", "Ctrl+U".yellow());
    println!("  {}    Delete word before cursor", "Ctrl+W".yellow());
    println!();
}

/// Print MCP server information
fn print_mcp_servers(mcp_manager: &Option<&mut McpManager>) {
    println!();
    if let Some(manager) = mcp_manager {
        let server_info = manager.get_server_info();
        if server_info.is_empty() {
            println!("{}", "No MCP servers configured.".dimmed());
        } else {
            println!("{}", "Configured MCP Servers".bold().underline());
            println!();
            for (name, tool_count, tools) in &server_info {
                println!(
                    "  {} {} ({} tools)",
                    "●".green(),
                    name.cyan().bold(),
                    tool_count
                );
                for tool in tools {
                    println!("    {} {}", "→".dimmed(), tool.dimmed());
                }
            }
        }

        // Show failed servers
        let failed = manager.get_failed_servers();
        if !failed.is_empty() {
            println!();
            println!("{}", "Failed Servers".red().bold());
            for (name, error) in &failed {
                println!("  {} {} - {}", "✗".red(), name.red(), error.dimmed());
            }
        }
    } else {
        println!("{}", "No MCP servers configured.".dimmed());
    }
    println!();
}

/// Result of reading a line with history support
enum LineResult {
    Line(String),
    Eof,
}

/// Read a line with history support (up/down arrows navigate history)
fn read_line_with_history(history: &[String], prompt: &str) -> io::Result<LineResult> {
    let mut stdout = io::stdout();

    // Print prompt
    print!("{}", prompt);
    stdout.flush()?;

    // Get the prompt row for absolute positioning during redraws
    let prompt_row = crossterm::cursor::position().unwrap_or((0, 0)).1;

    // Enable raw mode for key-by-key input
    enable_raw_mode()?;

    let mut buffer = String::new();
    let mut cursor_pos: usize = 0;
    let mut history_pos: usize = history.len(); // Start past the end (current input)
    let mut saved_input = String::new(); // Save current input when navigating history
    let mut showing_suggestions = false;
    let prompt_len = prompt.len() as u16;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match (code, modifiers) {
                    (KeyCode::Enter, _) => {
                        if showing_suggestions {
                            clear_suggestions(&mut stdout)?;
                        }
                        disable_raw_mode()?;
                        println!();
                        return Ok(LineResult::Line(buffer));
                    }
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+C - exit interactive mode
                        if showing_suggestions {
                            clear_suggestions(&mut stdout)?;
                        }
                        disable_raw_mode()?;
                        println!();
                        return Ok(LineResult::Eof);
                    }
                    (KeyCode::Char('d'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+D - EOF if buffer is empty
                        if buffer.is_empty() {
                            if showing_suggestions {
                                clear_suggestions(&mut stdout)?;
                            }
                            disable_raw_mode()?;
                            println!();
                            return Ok(LineResult::Eof);
                        }
                    }
                    (KeyCode::Tab, _) => {
                        // Tab - autocomplete slash commands
                        let matches = get_matching_commands(&buffer);
                        if !matches.is_empty() {
                            buffer = matches[0].to_string();
                            cursor_pos = buffer.len();
                            redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                            // Update suggestions after completion
                            let new_matches = get_matching_commands(&buffer);
                            show_suggestions(&mut stdout, &new_matches, &mut showing_suggestions)?;
                        }
                    }
                    (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+A - move to start
                        cursor_pos = 0;
                        move_cursor_to(&mut stdout, cursor_pos, prompt_len, prompt_row)?;
                    }
                    (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+E - move to end
                        cursor_pos = buffer.len();
                        move_cursor_to(&mut stdout, cursor_pos, prompt_len, prompt_row)?;
                    }
                    (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+U - clear line
                        buffer.clear();
                        cursor_pos = 0;
                        redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                        show_suggestions(&mut stdout, &[], &mut showing_suggestions)?;
                    }
                    (KeyCode::Char('w'), m) if m.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+W - delete word before cursor
                        if cursor_pos > 0 {
                            let mut new_pos = cursor_pos;
                            // Skip trailing spaces
                            while new_pos > 0
                                && buffer.chars().nth(new_pos - 1) == Some(' ')
                            {
                                new_pos -= 1;
                            }
                            // Delete until space or start
                            while new_pos > 0
                                && buffer.chars().nth(new_pos - 1) != Some(' ')
                            {
                                new_pos -= 1;
                            }
                            buffer.drain(new_pos..cursor_pos);
                            cursor_pos = new_pos;
                            redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                            let matches = get_matching_commands(&buffer);
                            show_suggestions(&mut stdout, &matches, &mut showing_suggestions)?;
                        }
                    }
                    (KeyCode::Char(c), _) => {
                        // Insert character at cursor position
                        buffer.insert(cursor_pos, c);
                        cursor_pos += 1;
                        // Redraw from cursor position
                        redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                        // Update suggestions
                        let matches = get_matching_commands(&buffer);
                        show_suggestions(&mut stdout, &matches, &mut showing_suggestions)?;
                    }
                    (KeyCode::Backspace, _) => {
                        if cursor_pos > 0 {
                            buffer.remove(cursor_pos - 1);
                            cursor_pos -= 1;
                            redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                            let matches = get_matching_commands(&buffer);
                            show_suggestions(&mut stdout, &matches, &mut showing_suggestions)?;
                        }
                    }
                    (KeyCode::Delete, _) => {
                        if cursor_pos < buffer.len() {
                            buffer.remove(cursor_pos);
                            redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                            let matches = get_matching_commands(&buffer);
                            show_suggestions(&mut stdout, &matches, &mut showing_suggestions)?;
                        }
                    }
                    (KeyCode::Left, _) => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                            move_cursor_to(&mut stdout, cursor_pos, prompt_len, prompt_row)?;
                        }
                    }
                    (KeyCode::Right, _) => {
                        if cursor_pos < buffer.len() {
                            cursor_pos += 1;
                            move_cursor_to(&mut stdout, cursor_pos, prompt_len, prompt_row)?;
                        }
                    }
                    (KeyCode::Home, _) => {
                        cursor_pos = 0;
                        move_cursor_to(&mut stdout, cursor_pos, prompt_len, prompt_row)?;
                    }
                    (KeyCode::End, _) => {
                        cursor_pos = buffer.len();
                        move_cursor_to(&mut stdout, cursor_pos, prompt_len, prompt_row)?;
                    }
                    (KeyCode::Up, _) => {
                        if !history.is_empty() && history_pos > 0 {
                            // Save current input if we're just starting to navigate
                            if history_pos == history.len() {
                                saved_input = buffer.clone();
                            }
                            history_pos -= 1;
                            buffer = history[history_pos].clone();
                            cursor_pos = buffer.len();
                            redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                            let matches = get_matching_commands(&buffer);
                            show_suggestions(&mut stdout, &matches, &mut showing_suggestions)?;
                        }
                    }
                    (KeyCode::Down, _) => {
                        if history_pos < history.len() {
                            history_pos += 1;
                            if history_pos == history.len() {
                                // Restore saved input
                                buffer = saved_input.clone();
                            } else {
                                buffer = history[history_pos].clone();
                            }
                            cursor_pos = buffer.len();
                            redraw_line(&mut stdout, &buffer, cursor_pos, prompt_len, prompt_row)?;
                            let matches = get_matching_commands(&buffer);
                            show_suggestions(&mut stdout, &matches, &mut showing_suggestions)?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Move cursor to a specific position in the buffer (handles line wrapping)
fn move_cursor_to(
    stdout: &mut io::Stdout,
    cursor_pos: usize,
    prompt_len: u16,
    prompt_row: u16,
) -> io::Result<()> {
    let term_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    // Calculate absolute screen position for cursor
    let cursor_total = prompt_len as usize + cursor_pos;
    let cursor_line = cursor_total / term_width;
    let cursor_col = (cursor_total % term_width) as u16;

    // Move to absolute position (prompt_row + cursor_line, cursor_col)
    stdout.execute(MoveTo(cursor_col, prompt_row + cursor_line as u16))?;

    Ok(())
}

/// Redraw the current line (used after editing)
/// Handles multiline input by using absolute positioning from prompt_row
fn redraw_line(
    stdout: &mut io::Stdout,
    buffer: &str,
    cursor_pos: usize,
    prompt_len: u16,
    prompt_row: u16,
) -> io::Result<()> {
    let term_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);

    // Move to the start of input (right after prompt on prompt row)
    stdout.execute(MoveTo(prompt_len, prompt_row))?;

    // Clear from here to end of screen (handles all wrapped lines)
    stdout.execute(Clear(ClearType::FromCursorDown))?;

    // Print the buffer
    print!("{}", buffer);
    stdout.flush()?;

    // Now position the cursor at cursor_pos
    let cursor_total = prompt_len as usize + cursor_pos;
    let cursor_line = cursor_total / term_width;
    let cursor_col = (cursor_total % term_width) as u16;

    // Move to absolute cursor position
    stdout.execute(MoveTo(cursor_col, prompt_row + cursor_line as u16))?;

    Ok(())
}

/// Run interactive mode for multi-turn conversations
pub async fn interactive_mode(
    client: &Client,
    model: &str,
    initial_prompt: Option<&str>,
    tool_output_limit: usize,
    mut mcp_manager: Option<&mut McpManager>,
    orchestrator: Option<&AgentOrchestrator>,
    agent_name: Option<&str>,
    silent: bool,
    verbose: bool,
    dmn: bool,
    enable_image_tool: bool,
    enable_search_tool: bool,
) -> Result<()> {
    let mut conversation_history: Vec<Message> = Vec::new();
    let mut dmn_injected = false;
    let mut input_history: Vec<String> = Vec::new();

    // Wait for MCP servers to be ready before showing prompt
    if let Some(ref mut manager) = mcp_manager {
        if manager.has_pending_servers() {
            let names = manager.pending_server_names();
            display::debug(&format!("Waiting for MCP server(s) to initialize: {}", names.join(", ")), verbose);
            if !silent {
                println!("Starting MCP servers: {}...", names.join(", "));
            }
            manager.await_all_servers().await;
        } else {
            manager.await_all_servers().await;
        }
    }

    // Show model/MCP info once at startup
    if !silent {
        display::print_model_info(model, client.provider());

        if let Some(ref manager) = mcp_manager {
            let server_info = manager.get_server_info();
            display::print_mcp_info(&server_info);
        }

        if dmn {
            display::print_dmn_mode();
        }

        // Detect research mode: search enabled + multi-agent orchestration (not DMN)
        let research = !dmn && enable_search_tool && orchestrator.as_ref().map_or(false, |o| o.has_agents());
        if research {
            display::print_research_mode();
        }

        if let Some(name) = agent_name {
            eprintln!("🤖 Multi-Agent Mode: starting as '{}'", name);
        }
    }

    // Process initial prompt if provided
    if let Some(prompt) = initial_prompt {
        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Spawn escape key monitor
        let cancel_tx_clone = cancel_tx.clone();
        let escape_handle = tokio::spawn(async move {
            monitor_escape_key(cancel_tx_clone).await;
        });

        let result = run_prompt(
            client,
            model,
            prompt,
            tool_output_limit,
            &mut mcp_manager,
            orchestrator,
            agent_name,
            silent,
            verbose,
            dmn,
            enable_image_tool,
            enable_search_tool,
            &mut dmn_injected,
            &mut conversation_history,
            Some(cancel_rx),
        )
        .await;

        escape_handle.abort();
        // Ensure raw mode is disabled after aborting the escape monitor
        let _ = disable_raw_mode();

        if let Ok(true) = result {
            display::print_user_stopped();
        } else if let Err(e) = result {
            display::print_error(&format!("Agent error: {}", e));
        }

        // Add initial prompt to history
        input_history.push(prompt.to_string());
    }

    // Interactive loop
    loop {
        let input = match read_line_with_history(&input_history, "> ")? {
            LineResult::Line(s) => s,
            LineResult::Eof => break,
        };

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        // Handle slash commands
        if input.eq_ignore_ascii_case("/help") {
            print_help();
            continue;
        }

        if input.eq_ignore_ascii_case("/mcp") {
            print_mcp_servers(&mcp_manager);
            continue;
        }

        // Add to history (avoid duplicates of last entry, skip commands)
        if !input.starts_with('/') && input_history.last().map(|s| s.as_str()) != Some(input) {
            input_history.push(input.to_string());
        }

        // Create cancellation channel
        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Spawn escape key monitor
        let cancel_tx_clone = cancel_tx.clone();
        let escape_handle = tokio::spawn(async move {
            monitor_escape_key(cancel_tx_clone).await;
        });

        let result = run_prompt(
            client,
            model,
            input,
            tool_output_limit,
            &mut mcp_manager,
            orchestrator,
            agent_name,
            silent,
            verbose,
            dmn,
            enable_image_tool,
            enable_search_tool,
            &mut dmn_injected,
            &mut conversation_history,
            Some(cancel_rx),
        )
        .await;

        // Stop escape key monitoring
        escape_handle.abort();
        // Ensure raw mode is disabled after aborting the escape monitor
        let _ = disable_raw_mode();

        if let Ok(true) = result {
            display::print_user_stopped();
        } else if let Err(e) = result {
            display::print_error(&format!("Agent error: {}", e));
        }
    }

    Ok(())
}

/// Run a single prompt - either through orchestrator (multi-agent) or directly
/// Returns true if the prompt was cancelled by the user
async fn run_prompt(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    mcp_manager: &mut Option<&mut McpManager>,
    orchestrator: Option<&AgentOrchestrator>,
    agent_name: Option<&str>,
    silent: bool,
    verbose: bool,
    dmn: bool,
    enable_image_tool: bool,
    enable_search_tool: bool,
    dmn_injected: &mut bool,
    conversation_history: &mut Vec<Message>,
    cancel_rx: Option<watch::Receiver<bool>>,
) -> Result<bool> {
    // Use orchestrator if available (multi-agent mode)
    let use_multi_agent = orchestrator.is_some() && agent_name.is_some() && mcp_manager.is_some();

    if use_multi_agent {
        let orch = orchestrator.unwrap();
        let name = agent_name.unwrap();
        let manager = mcp_manager.as_mut().unwrap();
        orch.run_agent(
            client,
            model,
            name,
            prompt,
            None,
            manager,
            tool_output_limit,
            silent,
            verbose,
            0,
            None, // No caller for root agent
        )
        .await?;
        Ok(false) // Multi-agent mode doesn't support cancellation yet
    } else {
        // Single-agent mode (original behavior)
        let final_prompt = inject_dmn_instructions_if_needed(prompt, dmn_injected, dmn);

        // Enable compaction in DMN mode
        let compaction_config = if dmn {
            Some(CompactionConfig::default())
        } else {
            None
        };

        let result = run_agent_cancellable(
            client,
            model,
            &final_prompt,
            tool_output_limit,
            mcp_manager.as_deref_mut(),
            silent,
            verbose,
            conversation_history,
            enable_image_tool,
            enable_search_tool,
            cancel_rx,
            compaction_config,
        )
        .await?;
        Ok(result == AgentResult::Cancelled)
    }
}

/// Inject DMN instructions if in DMN mode and not already injected
fn inject_dmn_instructions_if_needed(
    prompt: &str,
    dmn_injected: &mut bool,
    dmn: bool,
) -> String {
    if dmn && !*dmn_injected {
        *dmn_injected = true;
        format!(
            "{}\n\n---\n\n# USER REQUEST\n\n{}\n\n---\n\nYou are now in DMN (Default Mode Network) autonomous batch mode. Execute the user request above completely using your available MCP tools. Do not stop for confirmation.",
            DMN_INSTRUCTIONS, prompt
        )
    } else {
        prompt.to_string()
    }
}

/// Monitor for escape key press and signal cancellation
async fn monitor_escape_key(cancel_tx: watch::Sender<bool>) {
    // Enable raw mode to capture key events
    if enable_raw_mode().is_err() {
        return;
    }

    loop {
        // Poll for events with a small timeout
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                // Check for Escape key or Ctrl+C
                if key_event.code == KeyCode::Esc
                    || (key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL))
                {
                    let _ = disable_raw_mode();
                    let _ = cancel_tx.send(true);
                    return;
                }
            }
        }

        // Yield to other tasks
        tokio::task::yield_now().await;
    }
}
