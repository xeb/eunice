# Browser CLI Design

## Overview

A standalone `browser` binary (third binary alongside `eunice` and `mcpz`) that exposes Chrome DevTools automation as scriptable CLI commands. Shares core code with `src/mcpz/servers/browser.rs`. Each command is standalone, outputs JSON, and can be piped/chained.

Session and credential state persists in `~/.eunice/mcpz/browser.db` (SQLite, plaintext, filesystem-permission protected).

## Binary Entry Point

Add to `Cargo.toml`:

```toml
[[bin]]
name = "browser"
path = "src/browser_main.rs"
```

## Command Structure

```
browser <command> [options]

Global flags:
  --json          Force JSON output (default when piped/non-TTY)
  --quiet         Suppress non-essential output
  --port PORT     Chrome DevTools port (default: 9222)
  --chrome PATH   Path to Chrome executable
  --verbose       Show debug info
```

### Browser Lifecycle

```bash
browser start [--headless] [--port 9222]
browser stop
browser status                     # Is Chrome running? What port? What page?
```

### Navigation

```bash
browser open <url> [--wait 3] [--tab TAB_ID]
browser reload [--hard]
browser back
browser forward
```

### Page Content

```bash
browser page                       # Get HTML of current page
browser page --markdown            # Get page as markdown
browser save <filepath>            # Save DOM to file
browser screenshot <filepath> [--full-page]
browser pdf <filepath> [--landscape] [--background]
```

### JavaScript

```bash
browser script <code>              # Execute JS, return result
browser script -f <file.js>        # Execute JS from file
```

### Tabs

```bash
browser tabs                       # List open tabs
browser tab new [url]              # Open new tab
browser tab close <id>             # Close tab by ID
```

### DOM Interaction

```bash
browser find <selector>            # Find element, return info
browser wait <selector> [--timeout 5000]
browser click <selector>
browser type <text> [--selector S]
browser key <key> [--modifiers "Ctrl+Shift"]
```

### Cookies

```bash
browser cookies [--domain example.com]
browser cookie set <name> <value> [--domain D] [--path /] [--secure] [--http-only] [--expires EPOCH]
```

### Credential Store

```bash
browser cred add <name> --url <pattern> [--username U] [--password P] [--token T] [--headers '{"k":"v"}'] [--cookies '{"k":"v"}']
browser cred list
browser cred get <name>
browser cred delete <name>
browser cred apply <name>          # Apply to current page (inject cookies/headers, fill login form)
```

### Session Store

```bash
browser session save <name>        # Snapshot current cookies + localStorage for domain
browser session list
browser session load <name>        # Restore cookies + localStorage
browser session delete <name>
```

## Output Format

When connected to a TTY (interactive), output is human-readable:

```
$ browser open https://example.com
Navigated to https://example.com

$ browser tabs
ID          URL                          TITLE
a1b2c3d4    https://example.com          Example Domain
e5f6g7h8    https://github.com           GitHub
```

When piped or `--json` flag, output is JSON:

```json
$ browser open https://example.com --json
{"success": true, "message": "Navigated to https://example.com", "data": {"url": "https://example.com"}}

$ browser tabs --json
{"success": true, "data": {"tabs": [{"id": "a1b2c3d4", "url": "https://example.com", "title": "Example Domain"}]}}
```

Exit codes: 0 = success, 1 = error, 2 = browser not running.

## SQLite Schema (`~/.eunice/mcpz/browser.db`)

```sql
-- Stored credentials (login bundles)
CREATE TABLE credentials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    url_pattern TEXT NOT NULL,         -- glob pattern: "*.github.com", "https://app.example.com/*"
    username TEXT,
    password TEXT,
    token TEXT,                        -- Bearer/API token
    headers TEXT,                      -- JSON object of custom headers
    cookies TEXT,                      -- JSON array of cookie objects
    notes TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Saved browser sessions (cookie/localStorage snapshots)
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    domain TEXT NOT NULL,
    cookies TEXT NOT NULL,             -- JSON array from Network.getCookies
    local_storage TEXT,                -- JSON object (key-value pairs)
    user_agent TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Browser state (port, PID, last URL, etc.)
CREATE TABLE state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT DEFAULT (datetime('now'))
);
```

### State keys

| Key | Value | Purpose |
|-----|-------|---------|
| `chrome_port` | `"9222"` | DevTools port of running Chrome |
| `chrome_pid` | `"12345"` | PID for lifecycle management |
| `last_url` | `"https://..."` | Last navigated URL |
| `user_data_dir` | `"/tmp/mcpz/..."` | Chrome profile directory |

## Code Sharing Architecture

```
src/mcpz/servers/browser.rs        -- Core: ChromeClient, BrowserServer, BrowserResult
  ^                                    (shared types and logic)
  |
  +-- src/mcpz/cli.rs              -- MCP server entry: `mcpz server browser`
  |
  +-- src/browser_main.rs          -- CLI binary entry: `browser <command>`
      |
      +-- src/browser/mod.rs       -- CLI-specific: clap args, output formatting
      +-- src/browser/commands.rs   -- Command dispatch (thin wrappers calling BrowserServer)
      +-- src/browser/db.rs         -- SQLite persistence (credentials, sessions, state)
      +-- src/browser/output.rs     -- JSON/table output formatting
```

### Key Refactoring

1. Make `BrowserServer` methods public (they're already on `&self`)
2. Extract `BrowserServerConfig` creation into a shared constructor
3. The CLI creates a `BrowserServer` instance with config loaded from DB state (port, chrome path)
4. Commands call `BrowserServer` methods directly, format the `BrowserResult`

### No New Dependencies

Uses existing crate deps: `clap`, `rusqlite`, `serde_json`, `colored`, `dirs`.

## File Structure

```
src/
├── browser_main.rs              -- Binary entry point (clap App)
├── browser/
│   ├── mod.rs                   -- Module exports
│   ├── commands.rs              -- Command dispatch: start, open, click, etc.
│   ├── db.rs                    -- SQLite CRUD for credentials, sessions, state
│   └── output.rs                -- Format BrowserResult as JSON or table
```

## Example Workflows

### Login and save session

```bash
browser start --headless
browser open https://app.example.com/login --wait 5
browser type "user@example.com" --selector "#email"
browser type "password123" --selector "#password"
browser click "#login-button"
browser wait ".dashboard" --timeout 10000
browser session save "example-app"
browser stop
```

### Restore session later

```bash
browser start --headless
browser open https://app.example.com --wait 2
browser session load "example-app"
browser reload
browser page --markdown
```

### Store and apply credentials

```bash
# Save credentials once
browser cred add github \
  --url "*.github.com" \
  --username "myuser" \
  --token "ghp_xxxxxxxxxxxx" \
  --headers '{"Authorization": "token ghp_xxxxxxxxxxxx"}'

# Apply later
browser start
browser open https://github.com
browser cred apply github
```

### Scripting pipeline

```bash
# Scrape a list of URLs
cat urls.txt | while read url; do
  browser open "$url" --wait 2
  browser page --markdown >> output.md
  echo "---" >> output.md
done
```

### CI/Testing usage

```bash
browser start --headless --port 9333
browser open http://localhost:3000 --port 9333
result=$(browser find "#error-message" --json --port 9333)
if echo "$result" | jq -e '.data.nodeId > 0' > /dev/null; then
  echo "FAIL: Error message visible"
  exit 1
fi
browser stop --port 9333
```

## Implementation Order

1. `src/browser/db.rs` - SQLite setup, state CRUD
2. `src/browser/output.rs` - JSON/table formatting
3. `src/browser_main.rs` + `src/browser/mod.rs` - Clap arg parsing
4. `src/browser/commands.rs` - Wire commands to `BrowserServer`
5. Credential commands (add/list/get/delete/apply)
6. Session commands (save/list/load/delete)

## `cred apply` Behavior

When applying credentials to a page:

1. **Cookies**: Inject via `Network.setCookie` for matching domain
2. **Headers**: Store for subsequent requests (via `Fetch.enable` + request interception)
3. **Token**: If URL matches, inject as `Authorization` header
4. **Username/Password**: If a login form is detected (`input[type=password]`), fill and submit
5. **Fallback**: If no form found, just inject cookies and reload

## Security Note

Credentials are stored in **plaintext** in `~/.eunice/mcpz/browser.db`. Security relies on:
- File permissions: created with `0600` (user read/write only)
- Directory permissions: `~/.eunice/` is user-only
- This is equivalent to how `~/.netrc`, `~/.aws/credentials`, etc. work

For sensitive environments, users should use `browser cred delete` after use or avoid storing high-value credentials.
