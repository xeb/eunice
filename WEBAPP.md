# Webapp Mode Specification

## Overview

`--webapp` starts an embedded web server that provides a browser-based interface to Eunice. Users can submit queries and watch tool calls and results stream in real-time, mirroring the CLI experience in a web UI.

**Design Philosophy**: "Terminal in the browser" - maintain Eunice's sophisticated simplicity aesthetic while enabling web-based interaction.

## Command Line Interface

### New Flag

```
--webapp    Start web server interface (default: 0.0.0.0:8811)
```

### Flag Compatibility

| Flag | Compatible with --webapp |
|------|-------------------------|
| `--dmn` | Yes - enables DMN mode for web queries |
| `--research` | Yes - enables research mode |
| `--agent <NAME>` | Yes - uses specified agent |
| `--config <FILE>` | Yes - loads MCP configuration |
| `--model <MODEL>` | Yes - uses specified model |
| `--verbose` | Yes - enables verbose logging to server console |
| `--silent` | No - webapp requires output |
| `--interact` | No - conflicting interaction modes |
| `--events` | No - webapp handles its own event streaming |
| `--help` | No - exits immediately |
| `--list-*` | No - exits immediately |

### Exit Behavior

When incompatible flags are specified with `--webapp`:
- `--interact`: Error message and exit
- `--events`: Error message and exit
- `--silent`: Error message and exit
- `--help`, `--list-models`, `--list-agents`, `--list-tools`, `--list-mcp-servers`: Execute normally (help/list takes precedence, webapp ignored)

## Configuration

### Config File Properties

Add to `eunice.toml` / `eunice.json`:

```toml
[webapp]
host = "0.0.0.0"    # Optional, default: "0.0.0.0"
port = 8811         # Optional, default: 8811
```

```json
{
  "webapp": {
    "host": "0.0.0.0",
    "port": 8811
  }
}
```

### Priority

1. Config file values (if present)
2. Defaults: `0.0.0.0:8811`

Note: No command-line override for host/port - use config file if customization needed.

## Architecture

### Dependencies

Add to `Cargo.toml`:

```toml
# Web server
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }

# Server-Sent Events
tokio-stream = "0.1"
futures = "0.3"
```

**Rationale**: axum is lightweight, async-native, and aligns with Eunice's minimal dependency philosophy. ~200KB additional binary size.

### Module Structure

```
src/
├── webapp/
│   ├── mod.rs          # Module exports
│   ├── server.rs       # Axum server setup, routes
│   ├── handlers.rs     # HTTP/SSE request handlers
│   └── assets.rs       # Embedded HTML/CSS/JS (include_str!)
```

### Data Flow

```
Browser                    Axum Server                   Agent Loop
   │                           │                              │
   │── POST /api/query ───────>│                              │
   │                           │── spawn agent task ─────────>│
   │<── SSE stream ────────────│<── channel messages ─────────│
   │   (tool calls, results,   │   (ToolCall, ToolResult,     │
   │    thinking, response)    │    Thinking, Response)       │
   │                           │                              │
```

### Event Types (SSE)

```typescript
// Server-Sent Events format
interface SSEEvent {
  event: "thinking" | "tool_call" | "tool_result" | "response" | "error" | "done";
  data: string; // JSON payload
}

// Event payloads
interface ThinkingEvent {
  elapsed_seconds: number;
}

interface ToolCallEvent {
  name: string;
  arguments: string; // JSON string
}

interface ToolResultEvent {
  name: string;
  result: string;
  truncated: boolean;
}

interface ResponseEvent {
  content: string;
}

interface ErrorEvent {
  message: string;
}
```

## API Endpoints

### `GET /`

Serves the embedded single-page application.

### `GET /api/status`

Returns server status and configuration.

```json
{
  "version": "0.2.29",
  "model": "gemini-3-pro-preview",
  "provider": "Gemini",
  "mode": "dmn",  // or "research", "standard"
  "agent": "root",  // or null
  "mcp_servers": ["shell", "filesystem"],
  "tools_count": 12
}
```

### `POST /api/query`

Submit a query. Returns SSE stream.

**Request:**
```json
{
  "prompt": "What files are in this directory?"
}
```

**Response:** SSE stream with events as described above.

### `POST /api/cancel`

Cancel the current running query.

**Response:**
```json
{
  "cancelled": true
}
```

## Web Interface Design

### Aesthetic Direction

**Tone**: Synth Minimal - 80s computing meets Dieter Rams. Clean white canvas with sharp geometry and neon accents. Distinctive, not another dark-mode dev tool.

**Inspiration**: Early Macintosh UI crispness, Tron interfaces, graph paper precision, synthesizer panel aesthetics.

**Color Palette** (CSS variables):
```css
:root {
  /* Base - clean and bright */
  --bg: #fafafa;              /* Off-white, easy on eyes */
  --bg-card: #ffffff;         /* Pure white cards */
  --bg-input: #f5f5f5;        /* Subtle input background */
  --text: #1a1a1a;            /* Near-black text */
  --text-muted: #888888;      /* Soft gray for secondary */
  --border: #e0e0e0;          /* Light borders */
  --border-strong: #1a1a1a;   /* Sharp black borders */

  /* Synth neon accents */
  --neon-cyan: #00e5ff;       /* Tool calls, interactive */
  --neon-magenta: #ff00ff;    /* Highlights, focus states */
  --neon-coral: #ff6b6b;      /* Errors, warnings */
  --neon-mint: #00ffaa;       /* Success, results */
  --neon-yellow: #ffee00;     /* Thinking, processing */

  /* Subtle texture */
  --grid-line: rgba(0,0,0,0.03);  /* Graph paper grid */
  --scanline: rgba(0,0,0,0.02);   /* Optional CRT effect */
}
```

**Typography**:
- Primary: `IBM Plex Mono` - geometric, 80s computer aesthetic, excellent readability
- Fallback: `Space Mono, SF Mono, Consolas, monospace`
- Letter-spacing: `0.02em` on headings for that precision feel
- No rounded, friendly fonts - sharp and technical

**Visual Elements**:
- **Grid background**: Subtle graph paper pattern (4px or 8px grid)
- **Borders**: Thin black (1px) with sharp corners - no border-radius
- **Focus states**: Neon glow effect (`box-shadow: 0 0 0 2px var(--neon-cyan)`)
- **Left-border accents**: Colored bars indicate message types
- **Scanline overlay**: Optional ultra-subtle horizontal lines (CRT nostalgia)

### Layout

```
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│  E U N I C E                              gemini-3  ◉ ready   │
│  ────────────────────────────────────────────────────────────  │
│                                                                │
│  ┃ > list all rust files in the project                       │
│  │                                                             │
│  ┃ ▌ Thinking... 2s                              [neon-yellow] │
│  │                                                             │
│  ┃ 🔧 shell_execute_command                         [neon-cyan]│
│  ┃    {"command": "find . -name '*.rs'"}                       │
│  │                                                             │
│  ┃ → src/main.rs                                   [neon-mint] │
│  ┃   src/agent.rs                                              │
│  ┃   src/client.rs                                             │
│  ┃   src/config.rs                                             │
│  ┃   ▼ 8 more files                                            │
│  │                                                             │
│  ┃ Found 12 Rust source files in the project.                  │
│  │                                                             │
│  ├─────────────────────────────────────────────────────────────│
│  │                                                             │
│  │  [ Enter your query...                           ] [SEND]   │
│  │                                                             │
│  ├─────────────────────────────────────────────────────────────│
│  │  DMN ◆ shell, filesystem ◆ 12 tools ◆ 0.8s                  │
│  └─────────────────────────────────────────────────────────────┘
```

### UI Components

1. **Header Bar**
   - "E U N I C E" - spaced letters, bold weight, black on white
   - Model badge: bordered pill with model name
   - Status indicator: filled circle (◉) with color state
   - Thin horizontal rule separator

2. **Message Feed** (scrollable, grid background)
   - **User queries**: Prefixed with `>`, full black text
   - **Thinking**: Yellow left-border, animated block cursor `▌`
   - **Tool calls**: Cyan left-border, tool name bold, args in muted gray
   - **Tool results**: Mint left-border, monospace output, collapsible
   - **AI responses**: No left-border, clean black text
   - **Errors**: Coral left-border, coral text

3. **Input Area**
   - Sharp-cornered input field with thin black border
   - Placeholder in muted gray
   - SEND button: black background, white text, sharp corners
   - On focus: neon-cyan glow around input
   - Cancel button appears during execution (coral border)

4. **Status Bar**
   - Compact, single line
   - Diamond separators (◆) between items
   - Mode badge (DMN/Research), server names, tool count, elapsed time

### Animations

- **Thinking cursor**: Blinking block cursor `▌` (500ms interval, CSS only)
- **Message appear**: Slide-in from left + fade (150ms, subtle)
- **Focus glow**: Transition on box-shadow (100ms)
- **Collapse/expand**: Height + opacity transition (200ms)
- **Grid pulse**: Optional subtle grid-line pulse on new messages

### Responsive Behavior

- Min width: 320px (mobile usable)
- Optimal: 800px+
- Max content width: 1000px (centered)
- Full-height layout with sticky input

### Accessibility

- Semantic HTML (`<main>`, `<article>`, `<form>`)
- ARIA labels for interactive elements
- Keyboard navigation (Tab, Enter, Escape to cancel)
- High contrast colors (WCAG AA compliant)
- Reduced motion: disable spinner animation if `prefers-reduced-motion`

## Implementation Details

### Server Startup Flow

```rust
// In main.rs, after arg parsing
if args.webapp {
    // Validate incompatible flags
    if args.interact {
        return Err(anyhow!("--webapp and --interact cannot be used together"));
    }
    if args.events {
        return Err(anyhow!("--webapp and --events cannot be used together"));
    }
    if args.silent {
        return Err(anyhow!("--webapp and --silent cannot be used together"));
    }

    // Load webapp config
    let webapp_config = mcp_config
        .as_ref()
        .and_then(|c| c.webapp.as_ref())
        .cloned()
        .unwrap_or_default();

    // Start server (blocks until Ctrl+C)
    webapp::run_server(
        webapp_config,
        client,
        provider_info,
        mcp_manager,
        orchestrator,
        agent_name,
        args.tool_output_limit,
        args.verbose,
        args.dmn,
        args.research,
        args.dmn || args.images,
        args.dmn || args.search || args.research,
    ).await?;

    return Ok(());
}
```

### State Management

```rust
// Shared state across requests
pub struct AppState {
    pub client: Arc<Client>,
    pub provider_info: ProviderInfo,
    pub mcp_manager: Arc<Mutex<Option<McpManager>>>,
    pub orchestrator: Option<Arc<AgentOrchestrator>>,
    pub agent_name: Option<String>,
    pub tool_output_limit: usize,
    pub verbose: bool,
    pub dmn: bool,
    pub research: bool,
    pub enable_image_tool: bool,
    pub enable_search_tool: bool,
    // Active query cancellation
    pub cancel_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
}
```

### SSE Streaming

```rust
// Convert agent events to SSE
async fn stream_agent_response(
    state: Arc<AppState>,
    prompt: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let (tx, rx) = mpsc::channel(100);

    // Spawn agent task
    tokio::spawn(async move {
        // Run agent with event callback
        // Send events via tx
    });

    // Convert channel to SSE stream
    ReceiverStream::new(rx).map(|event| {
        Ok(Event::default()
            .event(event.event_type)
            .data(serde_json::to_string(&event.data).unwrap()))
    })
}
```

### Embedded Assets

```rust
// src/webapp/assets.rs
pub const INDEX_HTML: &str = include_str!("../../webapp/index.html");
pub const STYLES_CSS: &str = include_str!("../../webapp/styles.css");
pub const APP_JS: &str = include_str!("../../webapp/app.js");

// Or single embedded HTML with inline CSS/JS
pub const INDEX_HTML: &str = include_str!("../../webapp/index.html");
```

**Recommendation**: Single HTML file with embedded CSS/JS for simplicity. Approximately 300-400 lines total.

## Line Count Estimate

| Component | Lines | Notes |
|-----------|-------|-------|
| `src/webapp/mod.rs` | 10 | Module exports |
| `src/webapp/server.rs` | 150 | Server setup, routes, state |
| `src/webapp/handlers.rs` | 200 | Query handling, SSE streaming |
| `src/webapp/assets.rs` | 20 | Asset embedding |
| `webapp/index.html` | 350 | HTML + embedded CSS + JS |
| `src/models.rs` additions | 15 | WebappConfig struct |
| `src/config.rs` additions | 10 | Config parsing |
| `src/main.rs` additions | 50 | New flag, validation, startup |
| **Total** | **~805** | |

### Breakdown by Category

- **Rust server code**: ~430 lines
- **Web frontend**: ~350 lines
- **Config/integration**: ~75 lines

### Binary Size Impact

- axum + tower-http: ~300KB
- tokio-stream + futures: ~50KB (partial overlap with existing)
- **Total**: ~350KB additional (4.7MB → 5.0MB)

## Testing

### Manual Testing Checklist

1. Start server: `eunice --webapp --dmn`
2. Open `http://localhost:8811`
3. Verify status bar shows model, mode, tools
4. Submit query: "What files are in this directory?"
5. Verify thinking spinner appears
6. Verify tool calls stream in real-time
7. Verify tool results are displayed
8. Verify response is rendered
9. Test cancel button during execution
10. Test with --research mode
11. Test with custom --agent
12. Test custom port via config

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_webapp_config_defaults() {
        let config = WebappConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8811);
    }

    #[test]
    fn test_webapp_config_parsing() {
        let toml = r#"
        [webapp]
        host = "127.0.0.1"
        port = 9000
        "#;
        let config: McpConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.webapp.unwrap().port, 9000);
    }
}
```

## Security Considerations

1. **Bind Address**: Default `0.0.0.0` exposes to network - documented behavior
2. **No Authentication**: Webapp assumes trusted local network
3. **CORS**: Disabled (same-origin only by default)
4. **Input Validation**: Prompts are passed directly to agent (same as CLI)
5. **Resource Limits**: Consider max concurrent queries (1 for simplicity)

## Future Enhancements (Out of Scope)

- Multiple concurrent sessions
- Authentication/API keys
- Custom themes
- Conversation history persistence
- File upload for image interpretation
- Mobile-optimized layout

## Version

This specification is for eunice v0.2.29.
