//! TUI application using r3bl_tui's readline_async for proper output coordination.

use crate::agent::{self, AgentResult};
use crate::client::Client;
use crate::compact;
use crate::config::DMN_INSTRUCTIONS;
use crate::display_sink::TuiDisplaySink;
use crate::mcp::McpManager;
use crate::models::Message;
use crate::orchestrator::AgentOrchestrator;
use crate::models::ProviderInfo;

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
use std::time::Duration;
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
    agent_name: Option<&str>,
) -> std::io::Result<()> {
    write!(shared_writer, "  {DIM}")?;
    write!(shared_writer, "{PURPLE}model:{RESET}{DIM} {model}")?;
    write!(shared_writer, "  {PURPLE}tools:{RESET}{DIM} {tool_count}")?;
    if let Some(agent) = agent_name {
        write!(shared_writer, "  {PURPLE}agent:{RESET}{DIM} {agent}")?;
    }
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
        "\n{DIM}Type / for command menu, Ctrl+D to exit, Esc/Ctrl+C to stop generation{RESET}"
    )?;
    writeln!(
        shared_writer,
        "{DIM}Multiline paste supported via terminal paste (right-click, Ctrl+Shift+V){RESET}\n"
    )?;
    Ok(())
}

/// Simple line editor state for paste-enabled readline
struct LineEditor {
    line: String,
    cursor: usize,
}

impl LineEditor {
    fn new() -> Self {
        Self {
            line: String::new(),
            cursor: 0,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.line.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn insert_str(&mut self, s: &str) {
        self.line.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the previous character boundary
            let mut new_cursor = self.cursor - 1;
            while new_cursor > 0 && !self.line.is_char_boundary(new_cursor) {
                new_cursor -= 1;
            }
            self.line.drain(new_cursor..self.cursor);
            self.cursor = new_cursor;
        }
    }

    fn delete(&mut self) {
        if self.cursor < self.line.len() {
            // Find the next character boundary
            let mut end = self.cursor + 1;
            while end < self.line.len() && !self.line.is_char_boundary(end) {
                end += 1;
            }
            self.line.drain(self.cursor..end);
        }
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            let mut new_cursor = self.cursor - 1;
            while new_cursor > 0 && !self.line.is_char_boundary(new_cursor) {
                new_cursor -= 1;
            }
            self.cursor = new_cursor;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.line.len() {
            let mut new_cursor = self.cursor + 1;
            while new_cursor < self.line.len() && !self.line.is_char_boundary(new_cursor) {
                new_cursor += 1;
            }
            self.cursor = new_cursor;
        }
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.line.len();
    }

    fn clear(&mut self) {
        self.line.clear();
        self.cursor = 0;
    }

    fn delete_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // Skip trailing spaces
        let mut pos = self.cursor;
        while pos > 0 {
            let prev = {
                let mut p = pos - 1;
                while p > 0 && !self.line.is_char_boundary(p) {
                    p -= 1;
                }
                p
            };
            if self.line[prev..pos].trim().is_empty() {
                pos = prev;
            } else {
                break;
            }
        }
        // Skip the word
        while pos > 0 {
            let prev = {
                let mut p = pos - 1;
                while p > 0 && !self.line.is_char_boundary(p) {
                    p -= 1;
                }
                p
            };
            if !self.line[prev..pos].trim().is_empty() {
                pos = prev;
            } else {
                break;
            }
        }
        self.line.drain(pos..self.cursor);
        self.cursor = pos;
    }

    fn take_line(&mut self) -> String {
        let line = std::mem::take(&mut self.line);
        self.cursor = 0;
        line
    }
}

/// Render the prompt and current line directly to stdout
/// We use crossterm directly because SharedWriter doesn't handle
/// in-place line editing with escape sequences properly.
fn render_line(editor: &LineEditor) -> std::io::Result<()> {
    let mut stdout = std::io::stdout();

    // Move to beginning of line and clear it
    execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine))?;

    // Print prompt and line
    write!(stdout, "{PURPLE}>{RESET} {}", editor.line)?;

    // Move cursor to correct position
    let prompt_len = 2; // "> "
    let display_cursor = prompt_len + editor.line[..editor.cursor].chars().count();
    execute!(stdout, cursor::MoveToColumn(display_cursor as u16))?;

    stdout.flush()
}

/// Custom readline with bracketed paste support
///
/// Uses crossterm directly for line editing (SharedWriter doesn't handle
/// in-place editing properly). SharedWriter is still used for final output.
async fn read_line_with_paste(_ctx: &mut ReadlineAsyncContext) -> Result<ReadlineEvent> {
    let mut editor = LineEditor::new();

    // Initial render
    render_line(&editor)?;

    loop {
        // Poll for events with a small timeout
        if event::poll(Duration::from_millis(50)).map_err(|e| anyhow!("Poll error: {}", e))? {
            let ev = event::read().map_err(|e| anyhow!("Read error: {}", e))?;

            match ev {
                // Handle bracketed paste - this is the key addition
                Event::Paste(text) => {
                    // For multiline paste, join lines with spaces to preserve content
                    let processed = text.lines().collect::<Vec<_>>().join(" ");
                    editor.insert_str(&processed);
                    render_line(&editor)?;
                }

                Event::Key(key_event) => {
                    // Handle control keys
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        match key_event.code {
                            KeyCode::Char('c') => {
                                // Clear line and print ^C, then newline
                                let mut stdout = std::io::stdout();
                                execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                                writeln!(stdout, "{PURPLE}>{RESET} ^C")?;
                                return Ok(ReadlineEvent::Interrupted);
                            }
                            KeyCode::Char('d') => {
                                if editor.line.is_empty() {
                                    println!();
                                    return Ok(ReadlineEvent::Eof);
                                }
                                // If there's content, Ctrl+D does nothing
                            }
                            KeyCode::Char('a') => {
                                editor.move_home();
                                render_line(&editor)?;
                            }
                            KeyCode::Char('e') => {
                                editor.move_end();
                                render_line(&editor)?;
                            }
                            KeyCode::Char('u') => {
                                editor.clear();
                                render_line(&editor)?;
                            }
                            KeyCode::Char('w') => {
                                editor.delete_word_backward();
                                render_line(&editor)?;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match key_event.code {
                        KeyCode::Enter => {
                            let line = editor.take_line();
                            // Print the final line and move to next line
                            let mut stdout = std::io::stdout();
                            execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                            writeln!(stdout, "{PURPLE}>{RESET} {}", line)?;
                            return Ok(ReadlineEvent::Line(line));
                        }
                        KeyCode::Char(c) => {
                            editor.insert_char(c);
                            render_line(&editor)?;
                        }
                        KeyCode::Backspace => {
                            editor.backspace();
                            render_line(&editor)?;
                        }
                        KeyCode::Delete => {
                            editor.delete();
                            render_line(&editor)?;
                        }
                        KeyCode::Left => {
                            editor.move_left();
                            render_line(&editor)?;
                        }
                        KeyCode::Right => {
                            editor.move_right();
                            render_line(&editor)?;
                        }
                        KeyCode::Home => {
                            editor.move_home();
                            render_line(&editor)?;
                        }
                        KeyCode::End => {
                            editor.move_end();
                            render_line(&editor)?;
                        }
                        KeyCode::Up | KeyCode::Down => {
                            // TODO: History navigation (would need history access)
                        }
                        _ => {}
                    }
                }

                Event::Resize(_, _) => {
                    render_line(&editor)?;
                    return Ok(ReadlineEvent::Resized);
                }

                _ => {}
            }
        }

        // Yield to other tasks
        tokio::task::yield_now().await;
    }
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

/// Run TUI mode with proper readline and output coordination
#[allow(clippy::too_many_arguments)]
pub async fn run_tui_mode(
    client: &Client,
    provider_info: &ProviderInfo,
    initial_prompt: Option<&str>,
    tool_output_limit: usize,
    mut mcp_manager: Option<&mut McpManager>,
    orchestrator: Option<&AgentOrchestrator>,
    agent_name: Option<&str>,
    silent: bool,  // Used in fallback to interactive mode; TUI uses SharedWriter
    verbose: bool,
    dmn: bool,
    enable_image_tool: bool,
    enable_search_tool: bool,
) -> Result<()> {
    // Create readline context with custom prompt
    let prompt = format!("{PURPLE}>{RESET} ");
    let maybe_ctx = ReadlineAsyncContext::try_new(Some(prompt))
        .await
        .map_err(|e| anyhow!("Failed to create readline context: {}", e))?;

    let Some(mut ctx) = maybe_ctx else {
        eprintln!("Terminal is not interactive. Falling back to standard mode.");
        return crate::interactive::interactive_mode(
            client,
            &provider_info.resolved_model,
            initial_prompt,
            tool_output_limit,
            mcp_manager,
            orchestrator,
            agent_name,
            silent,
            verbose,
            dmn,
            enable_image_tool,
            enable_search_tool,
        )
        .await;
    };

    let mut shared_writer = ctx.clone_shared_writer();

    // Print header and status
    print_header(&mut shared_writer)?;

    // Count tools
    let tool_count = if let Some(ref manager) = mcp_manager {
        let mut count = manager.get_tools().len();
        if enable_image_tool {
            count += 1;
        }
        if enable_search_tool {
            count += 1;
        }
        count
    } else {
        let mut count = 0;
        if enable_image_tool {
            count += 1;
        }
        if enable_search_tool {
            count += 1;
        }
        count
    };

    print_status(&mut shared_writer, &provider_info.resolved_model, tool_count, agent_name)?;

    if dmn {
        writeln!(
            shared_writer,
            "  {YELLOW}DMN Mode Active{RESET} - Autonomous execution enabled\n"
        )?;
    }

    print_help(&mut shared_writer)?;

    // Conversation history
    let mut conversation_history: Vec<Message> = Vec::new();

    // Process initial prompt if provided
    if let Some(prompt_text) = initial_prompt {
        writeln!(shared_writer, "{DIM}Processing initial prompt...{RESET}\n")?;
        process_prompt(
            &mut ctx,
            client,
            provider_info,
            prompt_text,
            tool_output_limit,
            &mut mcp_manager,
            orchestrator,
            agent_name,
            verbose,
            dmn,
            enable_image_tool,
            enable_search_tool,
            &mut conversation_history,
        )
        .await?;
    }

    // Main event loop - use custom readline with paste support
    loop {
        let event = read_line_with_paste(&mut ctx).await?;

        match event {
            ReadlineEvent::Line(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
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
                                agent_name,
                            )?;
                            writeln!(
                                sw,
                                "  {DIM}History: {} messages{RESET}\n",
                                conversation_history.len()
                            )?;
                        }
                        "/quit" | "/q" | "/exit" => {
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

                // Add to history
                ctx.readline.add_history_entry(input.to_string());

                // Process the prompt
                process_prompt(
                    &mut ctx,
                    client,
                    provider_info,
                    input,
                    tool_output_limit,
                    &mut mcp_manager,
                    orchestrator,
                    agent_name,
                    verbose,
                    dmn,
                    enable_image_tool,
                    enable_search_tool,
                    &mut conversation_history,
                )
                .await?;
            }
            ReadlineEvent::Eof | ReadlineEvent::Interrupted => {
                let mut sw = ctx.clone_shared_writer();
                writeln!(sw, "\n{DIM}Goodbye!{RESET}\n")?;
                break;
            }
            ReadlineEvent::Resized => {
                // Handle terminal resize - continue loop
                continue;
            }
        }
    }

    Ok(())
}

/// Monitor for Escape or Ctrl+C to cancel the running agent
async fn monitor_cancel_keys(cancel_tx: watch::Sender<bool>) {
    loop {
        // Poll for events with a small timeout
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                // Check for Escape key or Ctrl+C
                if key_event.code == KeyCode::Esc
                    || (key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL))
                {
                    let _ = cancel_tx.send(true);
                    return;
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
    tool_output_limit: usize,
    mcp_manager: &mut Option<&mut McpManager>,
    orchestrator: Option<&AgentOrchestrator>,
    agent_name: Option<&str>,
    verbose: bool,
    dmn: bool,
    enable_image_tool: bool,
    enable_search_tool: bool,
    conversation_history: &mut Vec<Message>,
) -> Result<()> {
    let mut shared_writer = ctx.clone_shared_writer();
    writeln!(shared_writer)?;

    // Create TuiDisplaySink using the SharedWriter for coordinated output
    // This is the key change - all display output goes through the SharedWriter
    let display: Arc<dyn crate::display_sink::DisplaySink> = Arc::new(
        TuiDisplaySink::new(ctx.clone_shared_writer(), verbose)
    );

    // Prepare the final prompt
    let final_prompt = if dmn {
        format!(
            "{}\n\n---\n\n# USER REQUEST\n\n{}\n\n---\n\nYou are now in DMN (Default Mode Network) autonomous batch mode. Execute the user request above completely using your available MCP tools. Do not stop for confirmation.",
            DMN_INSTRUCTIONS, prompt
        )
    } else {
        prompt.to_string()
    };

    // Enable compaction in DMN mode
    let compaction_config = if dmn {
        Some(compact::CompactionConfig::default())
    } else {
        None
    };

    // Create cancellation channel
    let (cancel_tx, cancel_rx) = watch::channel(false);

    // Spawn cancel key monitor
    let cancel_handle = tokio::spawn(async move {
        monitor_cancel_keys(cancel_tx).await;
    });

    // Run the agent with the TuiDisplaySink - all output is coordinated through SharedWriter
    let result = match (orchestrator, agent_name) {
        (Some(orch), Some(name)) if mcp_manager.is_some() => {
            // Multi-agent mode - returns Result<String, Error>
            // Note: Multi-agent mode doesn't support cancellation yet
            let manager = mcp_manager.as_deref_mut().unwrap();
            orch.run_agent(
                client,
                &provider_info.resolved_model,
                name,
                &final_prompt,
                None,
                manager,
                tool_output_limit,
                display,
                0,
                None,
            )
            .await
            .map(|_| None)
        }
        _ => {
            // Single-agent mode with cancellation support
            agent::run_agent_cancellable(
                client,
                &provider_info.resolved_model,
                &final_prompt,
                tool_output_limit,
                mcp_manager.as_deref_mut(),
                display,
                conversation_history,
                enable_image_tool,
                enable_search_tool,
                Some(cancel_rx),
                compaction_config,
            )
            .await
            .map(|r| if r == AgentResult::Cancelled { Some(true) } else { None })
        }
    };

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
