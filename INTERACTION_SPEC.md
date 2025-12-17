# Interactive Mode Display Refactor Specification

## Problem Statement

The current `--interact` mode has severe terminal display issues caused by concurrent async output. Multiple components write to stdout/stderr simultaneously without coordination, causing:

- Jumbled lines and overwritten text
- "Thinking..." indicator conflicting with tool output
- Agent invocation messages appearing mid-line
- Random spacing and blank lines
- Carriage returns (`\r`) clobbering other output

### Root Causes

1. **No centralized output management**: Multiple files write directly to stdout
   - `src/display.rs`: ThinkingSpinner, tool calls, tool results
   - `src/agent.rs`: Model responses
   - `src/orchestrator/orchestrator.rs`: Agent invocations, tool calls
   - `src/interactive.rs`: Prompts, help, MCP info

2. **Concurrent async tasks**: The ThinkingSpinner runs in a separate tokio task while the main agent loop continues

3. **Mixed output streams**: Inconsistent use of `print!`, `println!`, `eprint!`, `eprintln!`

4. **No output coordination**: When ThinkingSpinner is active, other output still writes freely

---

## Research: Rust Terminal UI Libraries

### 1. r3bl_tui (Recommended)

**Crate**: [r3bl_tui](https://crates.io/crates/r3bl_tui)
**Version**: 0.7.6 (August 2025)
**Docs**: [docs.rs/r3bl_tui](https://docs.rs/r3bl_tui/latest/r3bl_tui/)
**GitHub**: [r3bl-org/r3bl-open-core](https://github.com/r3bl-org/r3bl-open-core)

> Note: `r3bl_terminal_async` was archived on 2025-04-05 and its functionality merged into `r3bl_tui`.

Purpose-built for exactly our problem: reading user input while concurrently writing output.

**Three Operational Modes**:
1. **Full TUI**: async, raw mode, full screen (overkill for us)
2. **Partial TUI - Choose API**: Interactive choice selection (`readline_async::choose_api`)
3. **Partial TUI - Readline**: Async readline for REPL apps (`readline_async::readline_async_api`)

**Key Features**:
- `SharedWriter` - thread-safe concurrent stdout writer (can be cloned)
- Built-in `Spinner` that **automatically pauses all SharedWriter output** when active
- Non-blocking async `read_line` with history support
- Tokio-native (same runtime we use)
- Customizable prompts

**Architecture**:
```
┌─────────────────────────────────────────────────┐
│              readline_async_api                  │
├──────────────────┬──────────────────────────────┤
│     Readline     │        SharedWriter(s)       │
│  (user input)    │    (concurrent output)       │
├──────────────────┼──────────────────────────────┤
│                  │  ┌─ Agent loop output        │
│   read_line()    │  ├─ Tool call output         │
│                  │  ├─ Tool result output       │
│                  │  └─ Spinner (auto-pauses)    │
└──────────────────┴──────────────────────────────┘
```

**Why this is ideal**:
- Spinner automatically pauses other writers (no conflicts!)
- All output goes through SharedWriter (centralized)
- Works with tokio (our runtime)
- Handles the readline + concurrent output problem natively
- Unlike POSIX readline which is single-threaded and blocking, this is fully async, interruptable, and non-blocking

**Key Quote from Docs**:
> "When the spinner is active, it pauses output to stdout, and resumes it when the spinner is stopped."

### 2. indicatif (Current Partial Usage)

**Crate**: [indicatif](https://crates.io/crates/indicatif)
**Docs**: [docs.rs/indicatif](https://docs.rs/indicatif)

We already have indicatif as a dependency but use it incorrectly.

**Key Features**:
- `ProgressBar::suspend()` - Hides progress bar, runs closure, redraws
- `ProgressBar::println()` - Print above the progress bar safely
- `MultiProgress` - Manage multiple progress bars (but has tokio issues)
- `indicatif-log-bridge` - Integrate with log crate

**Correct Usage Pattern**:
```rust
use indicatif::ProgressBar;

let pb = ProgressBar::new_spinner();
pb.enable_steady_tick(Duration::from_millis(100));

// WRONG: This conflicts with the spinner
println!("Tool called: {}", tool_name);

// RIGHT: Use pb.println() or pb.suspend()
pb.println(format!("Tool called: {}", tool_name));
// OR
pb.suspend(|| {
    println!("Tool called: {}", tool_name);
});
```

**Problem with MultiProgress + Tokio**:
- `MultiProgress::join()` is blocking
- Need to spawn on `tokio::task::spawn_blocking`
- More complex coordination required

### 3. Ratatui (Overkill for Our Needs)

**Crate**: [ratatui](https://crates.io/crates/ratatui)
**Docs**: [ratatui.rs](https://ratatui.rs/)

Full TUI framework (fork of tui-rs). Excellent for complex UIs but overkill for a CLI with streaming output.

**When to use**: Dashboard applications, complex layouts, widgets
**Not ideal for**: Streaming text output with occasional progress indicators

### 4. crossterm (Already Using)

**Crate**: [crossterm](https://crates.io/crates/crossterm)

Low-level terminal control. We already use it for raw mode and cursor positioning.

**Issue**: Manual cursor manipulation is error-prone with concurrent output.

---

## Proposed Solution

### Option A: r3bl_tui readline_async (Recommended)

Replace the entire interactive mode output system with `r3bl_tui`'s readline_async module.

**Pros**:
- Purpose-built for our exact problem
- Spinner automatically coordinates with output
- Clean abstraction over concurrent writes
- Active development and maintenance
- Already consolidated from multiple crates into one

**Cons**:
- Larger dependency (~40 transitive deps)
- Learning curve for new API

**Implementation Outline**:

```rust
// src/terminal.rs (new file)
use r3bl_tui::readline_async::{readline_async_api, SharedWriter, Spinner};

pub struct InteractiveTerminal {
    readline: readline_async_api::Readline,
    writer: SharedWriter,
}

impl InteractiveTerminal {
    pub async fn new() -> Result<Self> {
        let (readline, writer) = readline_async_api::create_readline_with_shared_writer().await?;
        Ok(Self { readline, writer })
    }

    pub async fn read_line(&mut self, prompt: &str) -> Option<String> {
        self.readline.read_line(prompt).await
    }

    pub fn writer(&self) -> SharedWriter {
        self.writer.clone()
    }

    pub fn start_spinner(&self) -> Spinner {
        Spinner::new(&self.writer)
    }
}
```

**Usage in agent loop**:
```rust
// All output through the shared writer
let writer = terminal.writer();

// Spinner automatically pauses the writer
let spinner = terminal.start_spinner();
let response = client.chat_completion(...).await;
spinner.stop();

// Tool output through writer - safe because spinner stopped
writeln!(writer, "  → {}", tool_name)?;
writeln!(writer, "    {}", result)?;
```

### Option B: Proper indicatif Usage (Simpler, Less Ideal)

Fix our indicatif usage by:
1. Creating a global `ProgressBar` for the thinking spinner
2. Using `pb.println()` for all output while spinner is active
3. Using `pb.suspend()` when needed

**Pros**:
- Already have the dependency
- Minimal code changes

**Cons**:
- Need to pass ProgressBar reference everywhere
- Still manual coordination required
- MultiProgress + tokio is awkward

**Implementation**:

```rust
// src/display.rs
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct OutputManager {
    spinner: Option<ProgressBar>,
}

impl OutputManager {
    pub fn new() -> Self {
        Self { spinner: None }
    }

    pub fn start_thinking(&mut self) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.yellow} Thinking...")
            .unwrap());
        pb.enable_steady_tick(Duration::from_millis(100));
        self.spinner = Some(pb);
    }

    pub fn stop_thinking(&mut self) {
        if let Some(pb) = self.spinner.take() {
            pb.finish_and_clear();
        }
    }

    pub fn println(&self, msg: &str) {
        if let Some(ref pb) = self.spinner {
            pb.println(msg);
        } else {
            println!("{}", msg);
        }
    }
}
```

### Option C: Channel-Based Output (Custom Solution)

Create an MPSC channel for all output, with a dedicated writer task.

**Pros**:
- Full control
- No new dependencies

**Cons**:
- More code to write and maintain
- Reinventing the wheel

---

## Recommended Implementation Plan

### Phase 1: Add r3bl_tui

1. Add dependency to Cargo.toml:
   ```toml
   r3bl_tui = "0.7"
   ```

2. Create `src/terminal.rs` with `InteractiveTerminal` wrapper

3. Update `src/interactive.rs`:
   - Replace `read_line_with_history` with readline_async
   - Use `SharedWriter` for all output
   - Use built-in spinner

### Phase 2: Centralize All Output

1. Create `OutputContext` that holds the `SharedWriter`
2. Pass `OutputContext` to:
   - `run_agent()` / `run_agent_cancellable()`
   - `AgentOrchestrator::run_agent()`
   - All display functions

3. Update `src/display.rs`:
   - All functions take `&OutputContext` or `&SharedWriter`
   - Remove direct `println!` calls
   - Remove `ThinkingSpinner` struct entirely

### Phase 3: Update Agent and Orchestrator

1. `src/agent.rs`:
   - Accept `OutputContext` parameter
   - Use `writer.writeln()` instead of `println!`
   - Use spinner from OutputContext

2. `src/orchestrator/orchestrator.rs`:
   - Accept `OutputContext` parameter
   - All output through SharedWriter

### Phase 4: Non-Interactive Mode Fallback

For non-interactive usage (single prompt, piped output):
- Detect if stdout is a TTY
- Use simple `println!` when not interactive
- No spinners when output is piped

---

## API Design

### New Types

```rust
// src/output.rs

use r3bl_tui::readline_async::SharedWriter;

/// Thread-safe output context for coordinated terminal output
pub struct OutputContext {
    writer: Option<SharedWriter>,  // None for non-interactive
    silent: bool,
}

impl OutputContext {
    /// Create for interactive mode
    pub fn interactive(writer: SharedWriter) -> Self {
        Self { writer: Some(writer), silent: false }
    }

    /// Create for non-interactive mode (simple println)
    pub fn simple(silent: bool) -> Self {
        Self { writer: None, silent }
    }

    /// Write a line (respects spinner, silent mode)
    pub fn writeln(&self, msg: impl AsRef<str>) {
        if self.silent {
            return;
        }
        if let Some(ref writer) = self.writer {
            let _ = writeln!(writer, "{}", msg.as_ref());
        } else {
            println!("{}", msg.as_ref());
        }
    }

    /// Print tool call
    pub fn tool_call(&self, name: &str) {
        self.writeln(format!("  → {}", name));
    }

    /// Print tool result
    pub fn tool_result(&self, result: &str, limit: usize) {
        let lines: Vec<&str> = result.lines().collect();
        let output = if limit > 0 && lines.len() > limit {
            let truncated: Vec<&str> = lines.iter().take(limit).cloned().collect();
            format!("{}\\n    ...{} more lines", truncated.join("\\n    "), lines.len() - limit)
        } else {
            result.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\\n")
        };
        self.writeln(output);
    }
}
```

### Updated Function Signatures

```rust
// src/agent.rs
pub async fn run_agent(
    client: &Client,
    model: &str,
    prompt: &str,
    tool_output_limit: usize,
    mcp_manager: Option<&mut McpManager>,
    output: &OutputContext,  // NEW
    verbose: bool,
    conversation_history: &mut Vec<Message>,
    enable_image_tool: bool,
    enable_search_tool: bool,
    compaction_config: Option<CompactionConfig>,
) -> Result<AgentResult>;

// src/orchestrator/orchestrator.rs
pub fn run_agent<'a>(
    &'a self,
    client: &'a Client,
    model: &'a str,
    agent_name: &'a str,
    task: &'a str,
    context: Option<&'a str>,
    mcp_manager: &'a mut McpManager,
    tool_output_limit: usize,
    output: &'a OutputContext,  // NEW
    verbose: bool,
    depth: usize,
    caller_agent: Option<&'a str>,
) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
```

---

## Spinner Integration

The key insight from r3bl_tui is that the spinner automatically pauses all SharedWriter output:

```rust
// src/terminal.rs

use r3bl_tui::readline_async::{Spinner, SharedWriter};

pub struct ThinkingIndicator {
    spinner: Spinner,
}

impl ThinkingIndicator {
    pub fn start(writer: &SharedWriter) -> Self {
        let spinner = Spinner::new(writer);
        spinner.start("Thinking...");
        Self { spinner }
    }
}

impl Drop for ThinkingIndicator {
    fn drop(&mut self) {
        self.spinner.stop();
    }
}

// Usage in agent loop:
{
    let _thinking = ThinkingIndicator::start(&output.writer);
    // All SharedWriter output is automatically paused here
    let response = client.chat_completion(...).await?;
    // Spinner stops on drop, output resumes
}

// Now safe to write tool output
output.tool_call(&tool_name);
output.tool_result(&result, limit);
```

---

## Testing Strategy

1. **Unit tests**: Test OutputContext formatting without terminal
2. **Integration tests**: Test interactive mode with mock terminal
3. **Manual testing**:
   - Multiple concurrent tool calls
   - Deep agent invocation (3+ levels)
   - Long-running operations with spinner
   - Escape key cancellation
   - Piped output (non-TTY)

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| r3bl_tui has breaking changes | Pin version `0.7`, test thoroughly |
| Binary size increase | Measure impact, acceptable tradeoff for correctness |
| Performance impact | Profile, r3bl uses double-buffering for efficiency |
| Learning curve | Document patterns, create examples |
| ~40 transitive dependencies | Acceptable for production-quality terminal handling |

---

## Comparison Summary

| Feature | r3bl_tui | indicatif | crossterm (current) |
|---------|----------|-----------|---------------------|
| Concurrent output coordination | Automatic | Manual (`suspend`/`println`) | None |
| Async readline | Yes (native) | No | Manual (raw mode) |
| Spinner pauses output | Automatic | Manual | No |
| Tokio integration | Native | Partial | None |
| Dependencies | ~40 | ~10 | ~5 |
| Maintenance | Active | Active | Active |

---

## Enhanced Full TUI Mode: "Hacker Aesthetic"

When `--interact` is enabled, we can leverage r3bl_tui's **Full TUI mode** for an immersive, multi-pane terminal experience that looks professional in tmux and feels like a proper hacker workstation.

### Design Vision

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ╔═══════════════════════════════════════════════════════════════════════╗  │
│  ║  E U N I C E                                              v0.2.48    ║  │
│  ╚═══════════════════════════════════════════════════════════════════════╝  │
├────────────────────────────────────┬────────────────────────────────────────┤
│  CONVERSATION                      │  RESPONSE                              │
│  ────────────────                  │  ────────────────                      │
│                                    │                                        │
│  ┌─ You ─────────────────────────┐ │  The fibonacci function can be        │
│  │ Write a fibonacci function    │ │  implemented recursively:             │
│  └───────────────────────────────┘ │                                        │
│                                    │  ```rust                               │
│  ┌─ Assistant ───────────────────┐ │  fn fib(n: u64) -> u64 {              │
│  │ Here's a fibonacci function   │ │      match n {                        │
│  │ with memoization...           │ │          0 => 0,                      │
│  └───────────────────────────────┘ │          1 => 1,                      │
│                                    │          _ => fib(n-1) + fib(n-2)     │
│  ┌─ You ─────────────────────────┐ │      }                                │
│  │ Can you optimize it?          │ │  }                                    │
│  └───────────────────────────────┘ │  ```                                  │
│                                    │                                        │
│                                    │  For better performance, use          │
│                                    │  memoization or iteration...          │
│                                    │                                        │
├────────────────────────────────────┴────────────────────────────────────────┤
│  TOOLS                                                                      │
│  ─────                                                                      │
│  ● shell_execute        ● filesystem_read      ● filesystem_write          │
│  ● browser_open_url     ● search_query         ● interpret_image           │
├─────────────────────────────────────────────────────────────────────────────┤
│  ◉ gemini-3-pro-preview (Gemini)  │  ⚡ 1,247 tokens  │  🔧 12 tools       │
│  ⋯ Thinking... 3s                 │  ↑↓ history       │  Tab: switch pane  │
└─────────────────────────────────────────────────────────────────────────────┘
> _
```

### Layout Components

Using r3bl_tui's **FlexBox layout system**:

```rust
// Root container - vertical stack
FlexBox::new()
    .direction(Direction::Column)
    .size(Size::Percentage(100))

    // Header with lolcat gradient
    .add_child(header_component)      // 3 rows fixed

    // Main content area - horizontal split
    .add_child(
        FlexBox::new()
            .direction(Direction::Row)
            .size(Size::Percentage(80))
            .add_child(conversation_pane)  // 40% width
            .add_child(response_pane)      // 60% width
    )

    // Tools panel
    .add_child(tools_panel)           // 4 rows fixed

    // Status bar
    .add_child(status_bar)            // 2 rows fixed

    // Input line (readline_async)
    .add_child(input_component)       // 1 row fixed
```

### Visual Features

#### 1. Lolcat Gradient Header

r3bl_tui includes a **lolcat color-wheel** implementation for rainbow gradients:

```rust
use r3bl_tui::lolcat::{ColorWheel, ColorWheelConfig};

fn render_header(surface: &mut Surface) {
    let config = ColorWheelConfig::default();
    let mut wheel = ColorWheel::new(config);

    // "EUNICE" with rainbow gradient
    let styled_title = wheel.colorize_str("E U N I C E");
    // Renders with smooth color transitions
}
```

The gradient gracefully degrades:
- **Truecolor terminals**: Full 16M color rainbow
- **ANSI256**: 256-color approximation
- **Basic terminals**: Grayscale fallback

#### 2. Syntax-Highlighted Code Blocks

r3bl_tui uses **syntect** for code highlighting in responses:

```rust
use r3bl_tui::syntax_highlighting::{SyntaxHighlighter, Theme};

fn render_code_block(code: &str, language: &str) -> StyledText {
    let highlighter = SyntaxHighlighter::new(Theme::Dracula);
    highlighter.highlight(code, language)
}
```

Supported languages: Rust, Python, JavaScript, TypeScript, Go, Shell, JSON, TOML, YAML, Markdown, and 100+ more via syntect.

#### 3. Conversation Pane with Scrollback

The left pane shows conversation history with:
- **User messages**: Cyan border, "You" label
- **Assistant messages**: Green border, "Assistant" label
- **Tool calls**: Yellow inline indicator
- **Scrollable**: Up/Down or mouse wheel

```rust
struct ConversationPane {
    messages: Vec<ConversationMessage>,
    scroll_offset: usize,
    focused: bool,
}

impl Component for ConversationPane {
    fn render(&self, surface: &mut Surface, area: Rect) {
        for (i, msg) in self.messages.iter().skip(self.scroll_offset).enumerate() {
            let style = match msg.role {
                Role::User => Style::new().fg(Color::Cyan).border(BorderStyle::Rounded),
                Role::Assistant => Style::new().fg(Color::Green).border(BorderStyle::Rounded),
            };
            // Render message box with style
        }
    }
}
```

#### 4. Live Response Pane with Streaming

The right pane shows the current response with:
- **Streaming text**: Characters appear as they arrive
- **Markdown rendering**: Headers, lists, emphasis
- **Code blocks**: Syntax highlighted with language label
- **Auto-scroll**: Follows new content

#### 5. Tools Status Panel

Shows available MCP tools with status indicators:

```
● shell_execute     ● filesystem_read   ○ browser_start (not available)
◉ search_query      ● interpret_image   ● filesystem_write
```

- **●** Green: Available and ready
- **○** Gray: Not available (e.g., Chrome not installed)
- **◉** Yellow: Currently executing

#### 6. Status Bar with Live Metrics

```
◉ gemini-3-pro-preview (Gemini)  │  ⚡ 1,247 tokens  │  🔧 12 tools  │  ⏱ 2.3s
```

- **Model indicator**: With provider icon
- **Token counter**: Running total for session
- **Tool count**: Available tools
- **Response time**: Last request duration

#### 7. Agent Hierarchy View (Multi-Agent Mode)

When using multi-agent orchestration, show the call stack:

```
┌─ AGENTS ─────────────────────────────────────────┐
│  root → researcher → fetcher                     │
│  ════   ══════════   ═══════                     │
│  idle   thinking...  executing: browser_open_url │
└──────────────────────────────────────────────────┘
```

### Keyboard Navigation

| Key | Action |
|-----|--------|
| `Tab` | Switch focus between panes |
| `↑/↓` | Scroll in focused pane / input history |
| `Ctrl+L` | Clear conversation |
| `Ctrl+T` | Toggle tools panel |
| `Ctrl+H` | Toggle agent hierarchy |
| `Escape` | Cancel current request |
| `Ctrl+C` | Exit |
| `F1` | Help modal |

### Modal Dialogs

r3bl_tui supports **glass-layer modals** that overlay the TUI:

```rust
// Help modal (F1)
ModalDialog::new()
    .title("Keyboard Shortcuts")
    .content(help_text)
    .buttons(vec!["Close"])
    .show(surface);

// MCP Server selector
AutocompleteDialog::new()
    .title("Select MCP Server")
    .options(server_names)
    .on_select(|server| { /* reconnect */ })
    .show(surface);
```

### Implementation Architecture

```rust
// src/tui/mod.rs
pub mod app;
pub mod components;
pub mod styles;

// src/tui/app.rs
use r3bl_tui::*;

pub struct EuniceApp {
    // State
    conversation: Vec<Message>,
    current_response: String,
    tools: Vec<ToolStatus>,
    model_info: ModelInfo,

    // Components
    conversation_pane: ConversationPane,
    response_pane: ResponsePane,
    tools_panel: ToolsPanel,
    status_bar: StatusBar,
    input: InputComponent,

    // Focus management
    focus: HasFocus,
}

impl App for EuniceApp {
    fn app_render(&self, surface: &mut Surface) {
        // Build flexbox layout
        let root = FlexBox::new()
            .direction(Direction::Column)
            .add_child(self.render_header())
            .add_child(self.render_main_content())
            .add_child(self.render_tools_panel())
            .add_child(self.render_status_bar())
            .add_child(self.render_input());

        root.render(surface);
    }

    fn app_handle_input_event(&mut self, event: InputEvent) -> EventPropagation {
        match event {
            InputEvent::Key(Key::Tab) => {
                self.focus.cycle_next();
                EventPropagation::Consumed
            }
            InputEvent::Key(Key::F1) => {
                self.show_help_modal();
                EventPropagation::Consumed
            }
            _ => {
                // Route to focused component
                self.focus.current().handle_event(event)
            }
        }
    }
}
```

### Stylesheet System

Define consistent styling with r3bl_tui's CSS-like system:

```rust
// src/tui/styles.rs
use r3bl_tui::*;

pub fn create_stylesheet() -> TuiStylesheet {
    stylesheet! {
        // Header gradient
        "header" => {
            fg: lolcat_gradient(),
            bold: true,
            padding: (0, 2),
        },

        // Conversation messages
        "user_message" => {
            fg: Color::Cyan,
            border: BorderStyle::Rounded,
            border_color: Color::Cyan,
        },
        "assistant_message" => {
            fg: Color::Green,
            border: BorderStyle::Rounded,
            border_color: Color::Green,
        },

        // Code blocks
        "code_block" => {
            bg: Color::Rgb(30, 30, 46),  // Dark background
            fg: Color::Rgb(205, 214, 244),
            padding: (1, 2),
        },

        // Status bar
        "status_bar" => {
            bg: Color::Rgb(49, 50, 68),
            fg: Color::Rgb(166, 173, 200),
        },

        // Tool indicators
        "tool_available" => { fg: Color::Green },
        "tool_unavailable" => { fg: Color::DarkGray },
        "tool_executing" => { fg: Color::Yellow, bold: true },
    }
}
```

### CLI Flags for Interactive Modes

Three options for how to expose Full TUI vs simple readline:

**Option A: Separate flags (Recommended)**
```
-i, --interact      Simple readline mode (current behavior, fixed)
    --tui           Full TUI mode with panes and visuals
```

This is cleanest - each flag does one thing. Users who want the fancy experience use `--tui`, others use `-i` as before.

**Option B: Flag with values**
```
-i, --interact[=MODE]   Interactive mode
                        MODE: simple (default), full
```
Usage: `eunice -i` or `eunice -i=full` or `eunice --interact=full`

**Option C: Modifier flag**
```
-i, --interact      Interactive mode (Full TUI if supported)
    --no-tui        Disable Full TUI, use simple readline
```
Here `--no-tui` only makes sense with `-i`.

**Recommendation**: Option A with `--tui` as a separate mode. This keeps `-i/--interact` working exactly as before (just fixed), and adds `--tui` as an opt-in premium experience.

```rust
// In main.rs
if args.tui {
    // Full TUI mode - requires TTY
    if !atty::is(atty::Stream::Stdout) {
        return Err(anyhow!("--tui requires a terminal"));
    }
    tui::run_full_tui(config).await?;
} else if args.interact {
    // Simple readline mode (fixed output issues)
    interactive::interactive_mode(config).await?;
}
```

**CLI help would show:**
```
Modes:
      --dmn             Autonomous batch mode with MCP tools
      --research        Multi-agent research mode
      --webapp          Browser-based interface
  -i, --interact        Interactive readline mode
      --tui             Full TUI with panes, syntax highlighting [requires TTY]
```

### tmux Compatibility

The Full TUI works well in tmux because:
- **Double buffering**: No flicker during redraws
- **Diff-based rendering**: Only changed pixels update
- **Proper terminal detection**: Adapts to tmux's TERM settings
- **Mouse support**: Works with tmux mouse mode
- **256-color/truecolor**: Respects tmux color passthrough

### Graceful Degradation

| Terminal | Experience |
|----------|------------|
| Modern (iTerm2, Alacritty, Kitty) | Full TUI with truecolor, mouse |
| tmux | Full TUI with 256 colors |
| SSH | Full TUI, diff rendering prevents flicker |
| Basic terminal | Full TUI with grayscale |
| Piped/non-TTY | Simple readline output only |

---

## Implementation Phases (Updated)

### Phase 1: Basic Full TUI Shell
- Implement `EuniceApp` with single-pane response view
- Add header with model info
- Add status bar with basic metrics
- Use readline_async for input

### Phase 2: Multi-Pane Layout
- Add conversation history pane (left)
- Add response pane (right) with markdown rendering
- Implement focus switching with Tab
- Add scrolling support

### Phase 3: Visual Polish
- Add lolcat gradient header
- Add syntax highlighting for code blocks
- Add tool status panel
- Implement keyboard shortcuts

### Phase 4: Advanced Features
- Add modal dialogs (help, server selection)
- Add agent hierarchy view for multi-agent mode
- Add mouse support
- Performance optimization

---

## References

- [r3bl_tui docs](https://docs.rs/r3bl_tui/latest/r3bl_tui/)
- [r3bl-org/r3bl-open-core GitHub](https://github.com/r3bl-org/r3bl-open-core)
- [r3bl_tui on lib.rs](https://lib.rs/crates/r3bl_tui)
- [edi - Markdown editor built with r3bl_tui](https://github.com/r3bl-org/r3bl-open-core)
- [r3bl_tui markdown parser](https://developerlife.com/2024/06/28/md-parser-rust-from-r3bl-tui/)
- [indicatif docs](https://docs.rs/indicatif)
- [indicatif suspend() PR](https://github.com/console-rs/indicatif/pull/333)
- [ratatui](https://ratatui.rs/)
- [CLI UX best practices](https://evilmartians.com/chronicles/cli-ux-best-practices-3-patterns-for-improving-progress-displays)
