mod browser;
mod mcpz;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use mcpz::servers::browser::{BrowserServer, BrowserServerConfig};
use std::path::PathBuf;
use std::process;

use browser::db::BrowserDb;
use browser::output::{format_error, format_result};

#[derive(Parser)]
#[command(name = "browser", about = "Chrome browser automation CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Chrome DevTools port
    #[arg(long, global = true, default_value = "9222")]
    port: u16,

    /// Path to Chrome executable
    #[arg(long, global = true)]
    chrome: Option<String>,

    /// Chrome user data directory (profile)
    #[arg(long, global = true)]
    user_data_dir: Option<String>,

    /// Force JSON output
    #[arg(long, global = true)]
    json: bool,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    quiet: bool,

    /// Enable verbose logging
    #[arg(long, global = true)]
    verbose: bool,

    /// Page load timeout in seconds
    #[arg(long, global = true, default_value = "30")]
    timeout: u64,
}

#[derive(Subcommand)]
enum Command {
    /// Start Chrome browser with remote debugging
    Start {
        /// Run Chrome in headless mode (no visible window)
        #[arg(long)]
        headless: bool,
    },
    /// Stop the running Chrome browser
    Stop,
    /// Check if Chrome is available
    Status,
    /// Navigate to a URL
    Open {
        /// URL to navigate to
        url: String,
        /// Seconds to wait after page load
        #[arg(long, default_value = "3")]
        wait: u64,
        /// Target tab ID
        #[arg(long)]
        tab: Option<String>,
    },
    /// Get current page content
    Page {
        /// Output as markdown instead of HTML
        #[arg(long)]
        markdown: bool,
    },
    /// Save page HTML to a file
    Save {
        /// File path to save to
        filepath: String,
    },
    /// Capture a screenshot
    Screenshot {
        /// File path to save screenshot
        filepath: String,
        /// Capture full page (not just viewport)
        #[arg(long)]
        full_page: bool,
    },
    /// Execute JavaScript in the browser
    Script {
        /// JavaScript code to execute
        code: Option<String>,
        /// Read script from file
        #[arg(short, long)]
        file: Option<String>,
    },
    /// List open browser tabs
    Tabs,
    /// Tab management
    Tab {
        #[command(subcommand)]
        action: TabAction,
    },
    /// Reload the current page
    Reload {
        /// Hard reload (bypass cache)
        #[arg(long)]
        hard: bool,
    },
    /// Navigate back in history
    Back,
    /// Navigate forward in history
    Forward,
    /// Get cookies for current page
    Cookies {
        /// Filter by domain
        #[arg(long)]
        domain: Option<String>,
    },
    /// Set a cookie
    Cookie {
        #[command(subcommand)]
        action: CookieAction,
    },
    /// Find an element by CSS selector
    Find {
        /// CSS selector
        selector: String,
    },
    /// Wait for an element to appear
    Wait {
        /// CSS selector
        selector: String,
        /// Timeout in milliseconds
        #[arg(long, default_value = "5000")]
        timeout: u64,
    },
    /// Click an element by CSS selector
    Click {
        /// CSS selector
        selector: String,
    },
    /// Type text into an element
    Type {
        /// Text to type
        text: String,
        /// CSS selector to focus first
        #[arg(long)]
        selector: Option<String>,
    },
    /// Press a keyboard key
    Key {
        /// Key name (Enter, Tab, Escape, etc.)
        key: String,
        /// Modifier keys (Ctrl+Shift, etc.)
        #[arg(long)]
        modifiers: Option<String>,
    },
    /// Save page as PDF
    Pdf {
        /// File path to save PDF
        filepath: String,
        /// Landscape orientation
        #[arg(long)]
        landscape: bool,
        /// Print background graphics
        #[arg(long)]
        background: bool,
    },
    /// Credential management
    Cred {
        #[command(subcommand)]
        action: CredAction,
    },
    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Subcommand)]
enum TabAction {
    /// Open a new tab
    New {
        /// URL to open
        url: Option<String>,
    },
    /// Close a tab by ID
    Close {
        /// Tab ID (from `browser tabs`)
        id: String,
    },
}

#[derive(Subcommand)]
enum CookieAction {
    /// Set a cookie
    Set {
        /// Cookie name
        name: String,
        /// Cookie value
        value: String,
        /// Cookie domain
        #[arg(long)]
        domain: Option<String>,
        /// Cookie path
        #[arg(long)]
        path: Option<String>,
        /// Secure flag
        #[arg(long)]
        secure: bool,
        /// HttpOnly flag
        #[arg(long)]
        http_only: bool,
        /// Expiration (epoch seconds)
        #[arg(long)]
        expires: Option<f64>,
    },
}

#[derive(Subcommand)]
enum CredAction {
    /// Add or update a credential
    Add {
        /// Credential name
        name: String,
        /// URL pattern to match
        #[arg(long)]
        url: String,
        /// Username
        #[arg(long)]
        username: Option<String>,
        /// Password
        #[arg(long)]
        password: Option<String>,
        /// Bearer/API token
        #[arg(long)]
        token: Option<String>,
        /// Custom headers (JSON object)
        #[arg(long)]
        headers: Option<String>,
        /// Cookies (JSON array)
        #[arg(long)]
        cookies: Option<String>,
        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// List stored credentials
    List,
    /// Get a credential by name
    Get {
        /// Credential name
        name: String,
    },
    /// Delete a credential
    Delete {
        /// Credential name
        name: String,
    },
    /// Apply credential to current page (inject cookies, fill forms)
    Apply {
        /// Credential name
        name: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// Save current page cookies as a session
    Save {
        /// Session name
        name: String,
    },
    /// List saved sessions
    List,
    /// Load a saved session (restore cookies)
    Load {
        /// Session name
        name: String,
    },
    /// Delete a saved session
    Delete {
        /// Session name
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {:#}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let json_mode = cli.json;

    match &cli.command {
        // Browser lifecycle commands
        Command::Start { headless } => {
            let server = create_server(&cli);
            let result = server.start_browser(*headless)?;
            let db = open_db()?;
            db.set_state("chrome_port", &cli.port.to_string())?;
            if let Some(ref dir) = cli.user_data_dir {
                db.set_state("user_data_dir", dir)?;
            }
            output(&result, json_mode);
            Ok(())
        }
        Command::Stop => {
            let server = create_server(&cli);
            let result = server.stop_browser()?;
            let db = open_db()?;
            db.delete_state("chrome_port")?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Status => {
            let server = create_server(&cli);
            let result = server.is_available();
            output(&result, json_mode);
            Ok(())
        }

        // Navigation
        Command::Open { url, wait, tab } => {
            let server = create_server(&cli);
            let result = server.open_url(url, *wait, tab.as_deref())?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Reload { hard } => {
            let server = create_server(&cli);
            let result = server.reload_page(*hard)?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Back => {
            let server = create_server(&cli);
            let result = server.go_back()?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Forward => {
            let server = create_server(&cli);
            let result = server.go_forward()?;
            output(&result, json_mode);
            Ok(())
        }

        // Page content
        Command::Page { markdown } => {
            let server = create_server(&cli);
            let result = if *markdown {
                server.get_page_as_markdown()?
            } else {
                server.get_page()?
            };
            output(&result, json_mode);
            Ok(())
        }
        Command::Save { filepath } => {
            let server = create_server(&cli);
            let result = server.save_page_contents(filepath)?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Screenshot { filepath, full_page } => {
            let server = create_server(&cli);
            let result = server.get_screenshot(filepath, *full_page)?;
            let br = mcpz::servers::browser::BrowserResult {
                success: true,
                message: format!("Screenshot saved to {}", result.path.display()),
                data: Some(serde_json::json!({
                    "path": result.path.display().to_string(),
                    "full_page": result.full_page,
                })),
            };
            output(&br, json_mode);
            Ok(())
        }
        Command::Pdf { filepath, landscape, background } => {
            let server = create_server(&cli);
            let result = server.print_to_pdf(filepath, *landscape, *background)?;
            output(&result, json_mode);
            Ok(())
        }

        // JavaScript
        Command::Script { code, file } => {
            let script = if let Some(path) = file {
                std::fs::read_to_string(path)
                    .map_err(|e| anyhow!("Failed to read script file '{}': {}", path, e))?
            } else if let Some(c) = code {
                c.clone()
            } else {
                return Err(anyhow!("Provide script code or --file"));
            };
            let server = create_server(&cli);
            let result = server.execute_script(&script)?;
            output(&result, json_mode);
            Ok(())
        }

        // Tabs
        Command::Tabs => {
            let server = create_server(&cli);
            let result = server.list_tabs()?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Tab { action } => {
            let server = create_server(&cli);
            match action {
                TabAction::New { url } => {
                    let result = server.new_tab(url.as_deref())?;
                    output(&result, json_mode);
                }
                TabAction::Close { id } => {
                    let result = server.close_tab(id)?;
                    output(&result, json_mode);
                }
            }
            Ok(())
        }

        // DOM interaction
        Command::Find { selector } => {
            let server = create_server(&cli);
            let result = server.find_element(selector)?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Wait { selector, timeout } => {
            let server = create_server(&cli);
            let result = server.wait_for_element(selector, *timeout)?;
            output(&result, json_mode);
            if !result.success {
                process::exit(1);
            }
            Ok(())
        }
        Command::Click { selector } => {
            let server = create_server(&cli);
            let result = server.click_element(selector)?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Type { text, selector } => {
            let server = create_server(&cli);
            let result = server.type_text(text, selector.as_deref())?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Key { key, modifiers } => {
            let server = create_server(&cli);
            let result = server.keyboard_press(key, modifiers.as_deref())?;
            output(&result, json_mode);
            Ok(())
        }

        // Cookies
        Command::Cookies { .. } => {
            let server = create_server(&cli);
            let result = server.get_cookies()?;
            output(&result, json_mode);
            Ok(())
        }
        Command::Cookie { action } => {
            let server = create_server(&cli);
            match action {
                CookieAction::Set { name, value, domain, path, secure, http_only, expires } => {
                    let result = server.set_cookie(
                        name, value, domain.as_deref(), path.as_deref(),
                        *secure, *http_only, *expires,
                    )?;
                    output(&result, json_mode);
                }
            }
            Ok(())
        }

        // Credentials
        Command::Cred { action } => handle_cred(action, &cli, json_mode),

        // Sessions
        Command::Session { action } => handle_session(action, &cli, json_mode),
    }
}

fn handle_cred(action: &CredAction, cli: &Cli, json_mode: bool) -> Result<()> {
    let db = open_db()?;

    match action {
        CredAction::Add { name, url, username, password, token, headers, cookies, notes } => {
            db.add_credential(
                name, url,
                username.as_deref(), password.as_deref(),
                token.as_deref(), headers.as_deref(),
                cookies.as_deref(), notes.as_deref(),
            )?;
            let msg = format!("Credential '{}' saved", name);
            if json_mode {
                println!("{}", serde_json::json!({"success": true, "message": msg}));
            } else {
                println!("{}", msg);
            }
        }
        CredAction::List => {
            let creds = db.list_credentials()?;
            if json_mode {
                let items: Vec<serde_json::Value> = creds.iter().map(|c| {
                    serde_json::json!({
                        "name": c.name,
                        "url_pattern": c.url_pattern,
                        "username": c.username,
                        "has_token": c.has_token,
                    })
                }).collect();
                println!("{}", serde_json::to_string(&serde_json::json!({"credentials": items}))?);
            } else if creds.is_empty() {
                println!("No credentials stored.");
            } else {
                println!("{:<16} {:<30} {:<16} {}", "NAME", "URL", "USERNAME", "TOKEN");
                for c in &creds {
                    println!("{:<16} {:<30} {:<16} {}",
                        c.name,
                        truncate(&c.url_pattern, 30),
                        c.username.as_deref().unwrap_or("-"),
                        if c.has_token { "yes" } else { "no" },
                    );
                }
            }
        }
        CredAction::Get { name } => {
            match db.get_credential(name)? {
                Some(c) => {
                    if json_mode {
                        println!("{}", serde_json::json!({
                            "name": c.name,
                            "url_pattern": c.url_pattern,
                            "username": c.username,
                            "password": c.password,
                            "token": c.token,
                            "headers": c.headers.as_ref().and_then(|h| serde_json::from_str::<serde_json::Value>(h).ok()),
                            "cookies": c.cookies.as_ref().and_then(|ck| serde_json::from_str::<serde_json::Value>(ck).ok()),
                            "notes": c.notes,
                        }));
                    } else {
                        println!("Name:     {}", c.name);
                        println!("URL:      {}", c.url_pattern);
                        if let Some(u) = &c.username { println!("Username: {}", u); }
                        if c.password.is_some() { println!("Password: ****"); }
                        if let Some(t) = &c.token { println!("Token:    {}...", &t[..t.len().min(12)]); }
                        if let Some(h) = &c.headers { println!("Headers:  {}", h); }
                        if let Some(ck) = &c.cookies { println!("Cookies:  {}", ck); }
                        if let Some(n) = &c.notes { println!("Notes:    {}", n); }
                    }
                }
                None => {
                    let msg = format!("Credential '{}' not found", name);
                    println!("{}", format_error(&msg, json_mode));
                    process::exit(1);
                }
            }
        }
        CredAction::Delete { name } => {
            if db.delete_credential(name)? {
                let msg = format!("Credential '{}' deleted", name);
                if json_mode {
                    println!("{}", serde_json::json!({"success": true, "message": msg}));
                } else {
                    println!("{}", msg);
                }
            } else {
                let msg = format!("Credential '{}' not found", name);
                println!("{}", format_error(&msg, json_mode));
                process::exit(1);
            }
        }
        CredAction::Apply { name } => {
            let cred = db.get_credential(name)?
                .ok_or_else(|| anyhow!("Credential '{}' not found", name))?;

            let server = create_server(cli);

            // Apply cookies if present
            if let Some(ref cookies_json) = cred.cookies {
                if let Ok(cookies) = serde_json::from_str::<Vec<serde_json::Value>>(cookies_json) {
                    for cookie in &cookies {
                        let cname = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        let domain = cookie.get("domain").and_then(|v| v.as_str());
                        let path = cookie.get("path").and_then(|v| v.as_str());
                        let secure = cookie.get("secure").and_then(|v| v.as_bool()).unwrap_or(false);
                        let http_only = cookie.get("httpOnly").and_then(|v| v.as_bool()).unwrap_or(false);
                        let _ = server.set_cookie(cname, value, domain, path, secure, http_only, None);
                    }
                }
            }

            // If username+password and there's a password input, try to fill the form
            if let (Some(ref user), Some(ref pass)) = (&cred.username, &cred.password) {
                let user_result = server.find_element("input[type=email], input[name=username], input[name=login], input#username, input#email");
                if let Ok(ref r) = user_result {
                    if r.success {
                        let _ = server.type_text(user, Some("input[type=email], input[name=username], input[name=login], input#username, input#email"));
                        let _ = server.type_text(pass, Some("input[type=password]"));
                    }
                }
            }

            let msg = format!("Credential '{}' applied", cred.name);
            if json_mode {
                println!("{}", serde_json::json!({"success": true, "message": msg}));
            } else {
                println!("{}", msg);
            }
        }
    }
    Ok(())
}

fn handle_session(action: &SessionAction, cli: &Cli, json_mode: bool) -> Result<()> {
    let db = open_db()?;

    match action {
        SessionAction::Save { name } => {
            let server = create_server(cli);
            let cookies_result = server.get_cookies()?;
            let cookies_json = cookies_result.data
                .as_ref()
                .and_then(|d| d.get("cookies"))
                .map(|c| c.to_string())
                .unwrap_or_else(|| "[]".to_string());

            // Try to get current URL for the domain
            let tabs_result = server.list_tabs()?;
            let domain = tabs_result.data
                .as_ref()
                .and_then(|d| d.get("tabs"))
                .and_then(|t| t.as_array())
                .and_then(|tabs| tabs.first())
                .and_then(|tab| tab.get("url"))
                .and_then(|u| u.as_str())
                .and_then(|url| url.split('/').nth(2))
                .unwrap_or("unknown")
                .to_string();

            db.save_session(name, &domain, &cookies_json, None, None)?;

            let msg = format!("Session '{}' saved ({}, {} cookies)", name, domain,
                serde_json::from_str::<Vec<serde_json::Value>>(&cookies_json)
                    .map(|v| v.len()).unwrap_or(0));
            if json_mode {
                println!("{}", serde_json::json!({"success": true, "message": msg}));
            } else {
                println!("{}", msg);
            }
        }
        SessionAction::List => {
            let sessions = db.list_sessions()?;
            if json_mode {
                let items: Vec<serde_json::Value> = sessions.iter().map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "domain": s.domain,
                        "created_at": s.created_at,
                    })
                }).collect();
                println!("{}", serde_json::to_string(&serde_json::json!({"sessions": items}))?);
            } else if sessions.is_empty() {
                println!("No sessions stored.");
            } else {
                println!("{:<20} {:<30} {}", "NAME", "DOMAIN", "CREATED");
                for s in &sessions {
                    println!("{:<20} {:<30} {}",
                        s.name,
                        s.domain,
                        s.created_at.as_deref().unwrap_or("-"),
                    );
                }
            }
        }
        SessionAction::Load { name } => {
            let session = db.get_session(name)?
                .ok_or_else(|| anyhow!("Session '{}' not found", name))?;

            let server = create_server(cli);

            // Parse and inject cookies
            if let Ok(cookies) = serde_json::from_str::<Vec<serde_json::Value>>(&session.cookies) {
                let mut count = 0;
                for cookie in &cookies {
                    let cname = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    let domain = cookie.get("domain").and_then(|v| v.as_str());
                    let path = cookie.get("path").and_then(|v| v.as_str());
                    let secure = cookie.get("secure").and_then(|v| v.as_bool()).unwrap_or(false);
                    let http_only = cookie.get("httpOnly").and_then(|v| v.as_bool()).unwrap_or(false);
                    let expires = cookie.get("expires").and_then(|v| v.as_f64());
                    if server.set_cookie(cname, value, domain, path, secure, http_only, expires).is_ok() {
                        count += 1;
                    }
                }
                let msg = format!("Session '{}' loaded ({} cookies injected)", name, count);
                if json_mode {
                    println!("{}", serde_json::json!({"success": true, "message": msg}));
                } else {
                    println!("{}", msg);
                }
            } else {
                return Err(anyhow!("Failed to parse session cookies"));
            }
        }
        SessionAction::Delete { name } => {
            if db.delete_session(name)? {
                let msg = format!("Session '{}' deleted", name);
                if json_mode {
                    println!("{}", serde_json::json!({"success": true, "message": msg}));
                } else {
                    println!("{}", msg);
                }
            } else {
                let msg = format!("Session '{}' not found", name);
                println!("{}", format_error(&msg, json_mode));
                process::exit(1);
            }
        }
    }
    Ok(())
}

/// Create a BrowserServer from CLI options
fn create_server(cli: &Cli) -> BrowserServer {
    let user_data_dir = cli.user_data_dir.as_ref().map(PathBuf::from);
    let config = BrowserServerConfig::with_user_data_dir(
        cli.port,
        cli.chrome.clone(),
        user_data_dir,
        cli.timeout,
        cli.verbose,
    );
    BrowserServer::new(config)
}

/// Open the browser database
fn open_db() -> Result<BrowserDb> {
    let db_path = dirs::home_dir()
        .ok_or_else(|| anyhow!("Cannot determine home directory"))?
        .join(".eunice")
        .join("mcpz")
        .join("browser.db");
    BrowserDb::open(&db_path)
}

/// Print a BrowserResult
fn output(result: &mcpz::servers::browser::BrowserResult, json_mode: bool) {
    println!("{}", format_result(result, json_mode));
    if !result.success {
        process::exit(1);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
