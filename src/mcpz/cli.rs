use super::http;
use super::servers;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use servers::browser::BrowserServerConfig;
use servers::filesystem::FilesystemServerConfig;
use servers::shell::ShellServerConfig;
use servers::sql::{AccessMode, DatabaseType, SqlServerConfig, connect_database};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Runtime MCP router tool for running MCP servers via npx, uvx, or cargo
#[derive(Parser)]
#[command(name = "mcpz")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Update mcpz from the git repository
    #[arg(long, global = true)]
    update: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an MCP server package
    Run {
        /// Package name (e.g., mcp-server-time, @modelcontextprotocol/server-filesystem)
        package: String,
        /// Automatically pick the first match (no prompt)
        #[arg(long, short = 'f')]
        first: bool,
        /// Additional arguments to pass to the package
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Search for an MCP package in npm/pip/crates.io registries (non-interactive)
    Search {
        /// Package name to search for
        package: String,
    },
    /// Search and pick a package to save to cache
    Pick {
        /// Package name to search for
        package: String,
    },
    /// Clear the package cache
    ClearCache,
    /// Run a built-in MCP server (shell, filesystem, sql, browser)
    #[command(after_help = "Available servers:\n  shell       Execute shell commands\n  filesystem  Filesystem operations\n  sql         SQL database queries\n  browser     Browser automation via Chrome DevTools\n\nRun 'mcpz server <SERVER> --help' for server-specific options.")]
    Server {
        /// List available built-in MCP servers
        #[arg(long, short = 'l')]
        list: bool,

        #[command(subcommand)]
        server_type: Option<ServerType>,
    },

    /// List cached package mappings and available servers
    List,
}

#[derive(Subcommand)]
enum ServerType {
    /// Start an MCP server for shell command execution
    Shell {
        /// Working directory for command execution
        #[arg(short = 'w', long, value_name = "PATH")]
        working_dir: Option<PathBuf>,

        /// Command execution timeout in seconds
        #[arg(short = 't', long, default_value = "30")]
        timeout: u64,

        /// Shell to use for command execution
        #[arg(short = 's', long, default_value = "/bin/sh")]
        shell: String,

        /// Only allow commands matching these patterns (comma-separated)
        #[arg(long, value_name = "PATTERNS")]
        allow: Option<String>,

        /// Deny commands matching these patterns (comma-separated)
        #[arg(long, value_name = "PATTERNS")]
        deny: Option<String>,

        /// Suppress stderr in command output
        #[arg(long)]
        no_stderr: bool,

        /// Enable verbose logging to stderr
        #[arg(short = 'v', long)]
        verbose: bool,

        // HTTP transport options
        /// Use HTTP transport instead of stdio
        #[arg(long)]
        http: bool,

        /// Port to listen on (HTTP only)
        #[arg(short = 'p', long, default_value = "3000")]
        port: u16,

        /// Address to bind to (HTTP only)
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Enable HTTPS (auto-generates self-signed cert if no --cert/--key)
        #[arg(long)]
        tls: bool,

        /// Path to TLS certificate (PEM format)
        #[arg(long, value_name = "PATH")]
        cert: Option<PathBuf>,

        /// Path to TLS private key (PEM format)
        #[arg(long, value_name = "PATH")]
        key: Option<PathBuf>,

        /// Allowed origins for CORS (comma-separated)
        #[arg(long, value_name = "ORIGINS")]
        origin: Option<String>,
    },

    /// Start an MCP server for filesystem operations
    Filesystem {
        /// Allowed directories (can specify multiple times, defaults to current directory)
        #[arg(short = 'd', long = "dir", value_name = "PATH")]
        allowed_directories: Vec<PathBuf>,

        /// Enable verbose logging to stderr
        #[arg(short = 'v', long)]
        verbose: bool,

        // HTTP transport options
        /// Use HTTP transport instead of stdio
        #[arg(long)]
        http: bool,

        /// Port to listen on (HTTP only)
        #[arg(short = 'p', long, default_value = "3000")]
        port: u16,

        /// Address to bind to (HTTP only)
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Enable HTTPS (auto-generates self-signed cert if no --cert/--key)
        #[arg(long)]
        tls: bool,

        /// Path to TLS certificate (PEM format)
        #[arg(long, value_name = "PATH")]
        cert: Option<PathBuf>,

        /// Path to TLS private key (PEM format)
        #[arg(long, value_name = "PATH")]
        key: Option<PathBuf>,

        /// Allowed origins for CORS (comma-separated)
        #[arg(long, value_name = "ORIGINS")]
        origin: Option<String>,
    },

    /// Start an MCP server for SQL database queries
    #[command(after_help = r#"EXAMPLES:
    # PostgreSQL (readonly - only SELECT allowed)
    mcpz server sql --connection postgres://user:pass@localhost:5432/mydb --readonly

    # MySQL with full access (SELECT, INSERT, UPDATE, DELETE)
    mcpz server sql --connection mysql://user:pass@localhost:3306/mydb --fullaccess

    # SQLite file database
    mcpz server sql --connection sqlite:///path/to/database.db --readonly

    # SQLite in-memory (for testing)
    mcpz server sql --connection sqlite::memory: --fullaccess

    # PostgreSQL over HTTPS
    mcpz server sql --connection postgres://user:pass@localhost/db --readonly --http --tls

SUPPORTED DATABASES:
    PostgreSQL  postgres://user:pass@host:5432/database
    MySQL       mysql://user:pass@host:3306/database
    MariaDB     mysql://user:pass@host:3306/database (uses MySQL protocol)
    SQLite      sqlite:///path/to/file.db or sqlite::memory:
"#)]
    Sql {
        /// Database connection string (required)
        #[arg(short = 'c', long, value_name = "URL", required = true)]
        connection: String,

        /// Read-only mode: only SELECT, SHOW, DESCRIBE allowed
        #[arg(long, conflicts_with = "fullaccess", required_unless_present = "fullaccess")]
        readonly: bool,

        /// Full access mode: all SQL statements allowed (INSERT, UPDATE, DELETE, etc.)
        #[arg(long, conflicts_with = "readonly", required_unless_present = "readonly")]
        fullaccess: bool,

        /// Query timeout in seconds
        #[arg(short = 't', long, default_value = "30")]
        timeout: u64,

        /// Enable verbose logging to stderr
        #[arg(short = 'v', long)]
        verbose: bool,

        // HTTP transport options
        /// Use HTTP transport instead of stdio
        #[arg(long)]
        http: bool,

        /// Port to listen on (HTTP only)
        #[arg(short = 'p', long, default_value = "3000")]
        port: u16,

        /// Address to bind to (HTTP only)
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Enable HTTPS (auto-generates self-signed cert if no --cert/--key)
        #[arg(long)]
        tls: bool,

        /// Path to TLS certificate (PEM format)
        #[arg(long, value_name = "PATH")]
        cert: Option<PathBuf>,

        /// Path to TLS private key (PEM format)
        #[arg(long, value_name = "PATH")]
        key: Option<PathBuf>,

        /// Allowed origins for CORS (comma-separated)
        #[arg(long, value_name = "ORIGINS")]
        origin: Option<String>,
    },

    /// Start an MCP server for browser automation via Chrome DevTools Protocol
    #[command(after_help = r#"EXAMPLES:
    # Start browser server on default port (9222)
    mcpz server browser

    # Start with custom Chrome DevTools port
    mcpz server browser -p 9333

    # Start with verbose logging
    mcpz server browser --verbose

    # Start with HTTP transport
    mcpz server browser --http --port 3000

TOOLS PROVIDED:
    start_browser       Start Chrome with remote debugging
    stop_browser        Stop the Chrome instance
    open_url           Navigate to a URL
    get_page           Get full HTML content
    get_page_as_markdown  Get page as markdown
    get_screenshot     Capture screenshot (viewport or full page)
    execute_script     Execute JavaScript in browser
"#)]
    Browser {
        /// Chrome DevTools Protocol port
        #[arg(short = 'p', long, default_value = "9222")]
        cdp_port: u16,

        /// Path to Chrome executable
        #[arg(long, value_name = "PATH")]
        chrome_path: Option<String>,

        /// Page load timeout in seconds
        #[arg(short = 't', long, default_value = "30")]
        timeout: u64,

        /// Enable verbose logging to stderr
        #[arg(short = 'v', long)]
        verbose: bool,

        // HTTP transport options
        /// Use HTTP transport instead of stdio
        #[arg(long)]
        http: bool,

        /// Port to listen on (HTTP only)
        #[arg(long, default_value = "3000")]
        port: u16,

        /// Address to bind to (HTTP only)
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Enable HTTPS (auto-generates self-signed cert if no --cert/--key)
        #[arg(long)]
        tls: bool,

        /// Path to TLS certificate (PEM format)
        #[arg(long, value_name = "PATH")]
        cert: Option<PathBuf>,

        /// Path to TLS private key (PEM format)
        #[arg(long, value_name = "PATH")]
        key: Option<PathBuf>,

        /// Allowed origins for CORS (comma-separated)
        #[arg(long, value_name = "ORIGINS")]
        origin: Option<String>,
    },
}

/// Determines the package type based on the package name
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    /// Cargo/Rust package (runs with cargo install)
    Cargo,
    /// Python package (runs with uvx)
    Python,
    /// npm package (runs with npx)
    Npm,
}

impl PackageType {
    /// Get the runner command for this package type
    pub fn runner(&self) -> &'static str {
        match self {
            PackageType::Npm => "npx",
            PackageType::Python => "uvx",
            PackageType::Cargo => "cargo",
        }
    }

    /// Get the installer instructions for this package type
    pub fn install_instructions(&self) -> &'static str {
        match self {
            PackageType::Npm => "Install Node.js/npm from https://nodejs.org/ or run: curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash - && sudo apt-get install -y nodejs",
            PackageType::Python => "Install uv by running: curl -LsSf https://astral.sh/uv/install.sh | sh",
            PackageType::Cargo => "Install Rust/Cargo from https://rustup.rs/ or run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
        }
    }

    /// Get display name for this package type
    pub fn display_name(&self) -> &'static str {
        match self {
            PackageType::Npm => "npm",
            PackageType::Python => "PyPI",
            PackageType::Cargo => "crates.io",
        }
    }
}

/// Information about a found package
#[derive(Debug, Clone)]
struct PackageInfo {
    name: String,
    version: String,
    description: String,
    author: String,
    published: String,
    downloads: Option<u64>,
    registry: PackageType,
}

impl PackageInfo {
    fn display(&self, index: usize) {
        println!(
            "{}",
            format!("[{}] {} v{}", index + 1, self.name, self.version)
                .green()
                .bold()
        );
        println!("    Registry:    {}", self.registry.display_name().cyan());
        println!("    Description: {}", self.description);
        println!("    Author:      {}", self.author);
        println!("    Published:   {}", self.published);
        if let Some(dl) = self.downloads {
            println!("    Downloads:   {}", format_downloads(dl).yellow());
        }
        println!();
    }
}

/// Format download count with K/M suffix
fn format_downloads(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Sort packages by download count (most popular first)
fn sort_by_popularity(packages: &mut [PackageInfo]) {
    packages.sort_by(|a, b| {
        match (b.downloads, a.downloads) {
            (Some(b_dl), Some(a_dl)) => b_dl.cmp(&a_dl),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
}

/// Package cache stored in ~/.cache/mcpz/package_mapping.toml
#[derive(Debug, Default, Serialize, Deserialize)]
struct PackageCache {
    /// Maps search term -> (actual package name, package type)
    packages: HashMap<String, (String, PackageType)>,
}

impl PackageCache {
    fn cache_path() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow!("Could not determine cache directory"))?
            .join("mcpz");
        Ok(cache_dir.join("package_mapping.toml"))
    }

    fn load() -> Result<Self> {
        let path = Self::cache_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path).context("Failed to read cache file")?;
        toml::from_str(&content).context("Failed to parse cache file")
    }

    fn save(&self) -> Result<()> {
        let path = Self::cache_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create cache directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize cache")?;
        fs::write(&path, content).context("Failed to write cache file")?;
        Ok(())
    }

    fn get(&self, search_term: &str) -> Option<(String, PackageType)> {
        self.packages.get(search_term).cloned()
    }

    fn set(&mut self, search_term: String, package_name: String, pkg_type: PackageType) {
        self.packages.insert(search_term, (package_name, pkg_type));
    }

    fn clear() -> Result<()> {
        let path = Self::cache_path()?;
        if path.exists() {
            fs::remove_file(&path).context("Failed to remove cache file")?;
        }
        Ok(())
    }
}

/// Check if a command exists on the system
pub fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get npm download count for a package
fn get_npm_downloads(client: &reqwest::blocking::Client, package: &str) -> Option<u64> {
    let url = format!(
        "https://api.npmjs.org/downloads/point/last-month/{}",
        package
    );
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: serde_json::Value = resp.json().ok()?;
    data.get("downloads").and_then(|v| v.as_u64())
}

/// Search npm registry and return matching packages
fn search_npm(query: &str) -> Vec<PackageInfo> {
    if !command_exists("npm") {
        return vec![];
    }

    let output = Command::new("npm")
        .args(["search", "--json", query])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let results: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok();

    let mut packages = vec![];
    if let Some(arr) = results.as_array() {
        for item in arr.iter().take(10) {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let version = item.get("version").and_then(|v| v.as_str()).unwrap_or("?");
            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("No description");
            let author = item
                .get("publisher")
                .and_then(|p| p.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let date = item.get("date").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let published = date.split('T').next().unwrap_or(date).to_string();

            // Get download count
            let downloads = client
                .as_ref()
                .and_then(|c| get_npm_downloads(c, name));

            if !name.is_empty() {
                packages.push(PackageInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                    description: description.to_string(),
                    author: author.to_string(),
                    published,
                    downloads,
                    registry: PackageType::Npm,
                });
            }
        }
    }

    packages
}

/// Get PyPI download count for a package (last month)
fn get_pypi_downloads(client: &reqwest::blocking::Client, package: &str) -> Option<u64> {
    let url = format!("https://pypistats.org/api/packages/{}/recent", package);
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: serde_json::Value = resp.json().ok()?;
    data.get("data")
        .and_then(|d| d.get("last_month"))
        .and_then(|v| v.as_u64())
}

/// Search PyPI registry and return matching packages
fn search_pypi(query: &str) -> Vec<PackageInfo> {
    // PyPI doesn't have a search API, so we'll check if the exact package exists
    // and also try common variations
    let variations = vec![
        query.to_string(),
        query.replace('-', "_"),
        query.replace('_', "-"),
    ];

    let mut packages = vec![];
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return packages,
    };

    for pkg_name in variations {
        let url = format!("https://pypi.org/pypi/{}/json", pkg_name);
        let resp = match client.get(&url).send() {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let data: serde_json::Value = match resp.json() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let info = match data.get("info") {
            Some(i) => i,
            None => continue,
        };

        let name = info.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let version = info.get("version").and_then(|v| v.as_str()).unwrap_or("?");
        let description = info
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("No description");
        let author = info
            .get("author")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                info.get("author_email")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split('<').next().unwrap_or(s).trim())
            })
            .unwrap_or("Unknown");

        // Get upload time from releases
        let published = data
            .get("urls")
            .and_then(|u| u.as_array())
            .and_then(|arr| arr.first())
            .and_then(|u| u.get("upload_time"))
            .and_then(|v| v.as_str())
            .map(|s| s.split('T').next().unwrap_or(s))
            .unwrap_or("Unknown")
            .to_string();

        // Get download count
        let downloads = get_pypi_downloads(&client, name);

        if !name.is_empty() && !packages.iter().any(|p: &PackageInfo| p.name == name) {
            packages.push(PackageInfo {
                name: name.to_string(),
                version: version.to_string(),
                description: description.to_string(),
                author: author.to_string(),
                published,
                downloads,
                registry: PackageType::Python,
            });
        }
    }

    packages
}

/// Search crates.io registry and return matching packages (using API for full details)
fn search_cargo(query: &str) -> Vec<PackageInfo> {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("mcpz")
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    // Use crates.io API for search
    let url = format!(
        "https://crates.io/api/v1/crates?q={}&per_page=10",
        urlencoding::encode(query)
    );

    let resp = match client.get(&url).send() {
        Ok(r) if r.status().is_success() => r,
        _ => return vec![],
    };

    let data: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut packages = vec![];

    if let Some(crates) = data.get("crates").and_then(|c| c.as_array()) {
        for item in crates.iter().take(10) {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let version = item
                .get("newest_version")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("No description");
            let downloads = item.get("downloads").and_then(|v| v.as_u64());
            let updated = item
                .get("updated_at")
                .and_then(|v| v.as_str())
                .map(|s| s.split('T').next().unwrap_or(s))
                .unwrap_or("Unknown")
                .to_string();

            if !name.is_empty() {
                packages.push(PackageInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                    description: description.to_string(),
                    author: "See crates.io".to_string(),
                    published: updated,
                    downloads,
                    registry: PackageType::Cargo,
                });
            }
        }
    }

    packages
}

/// Search all registries and let user pick a package
fn search_and_select(query: &str) -> Result<Option<(String, PackageType)>> {
    println!(
        "{}",
        format!("Searching for '{}' across all registries...", query).cyan()
    );
    println!();

    let mut all_packages = vec![];

    // Search cargo first
    print!("  Searching crates.io... ");
    std::io::stdout().flush()?;
    let cargo_results = search_cargo(query);
    println!("{} found", cargo_results.len());
    all_packages.extend(cargo_results);

    // Search PyPI
    print!("  Searching PyPI... ");
    std::io::stdout().flush()?;
    let pypi_results = search_pypi(query);
    println!("{} found", pypi_results.len());
    all_packages.extend(pypi_results);

    // Search npm
    print!("  Searching npm... ");
    std::io::stdout().flush()?;
    let npm_results = search_npm(query);
    println!("{} found", npm_results.len());
    all_packages.extend(npm_results);

    println!();

    if all_packages.is_empty() {
        println!(
            "{}",
            format!("No packages found matching '{}'", query).red()
        );
        return Ok(None);
    }

    // Sort by popularity (most downloads first)
    sort_by_popularity(&mut all_packages);

    // Display all packages
    println!(
        "{}",
        format!("Found {} packages (sorted by popularity):", all_packages.len())
            .green()
            .bold()
    );
    println!();

    for (i, pkg) in all_packages.iter().enumerate() {
        pkg.display(i);
    }

    // Let user select
    print!(
        "{}",
        "Select a package (1-{}) or 'q' to quit: "
            .replace("{}", &all_packages.len().to_string())
            .yellow()
    );
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.eq_ignore_ascii_case("q") {
        return Ok(None);
    }

    let selection: usize = input.parse().context("Invalid selection")?;
    if selection < 1 || selection > all_packages.len() {
        return Err(anyhow!("Selection out of range"));
    }

    let selected = &all_packages[selection - 1];
    println!();
    println!(
        "{}",
        format!(
            "Selected: {} ({}) from {}",
            selected.name,
            selected.version,
            selected.registry.display_name()
        )
        .green()
    );

    Ok(Some((selected.name.clone(), selected.registry)))
}

/// Discover package type by searching registries
/// If multiple exact matches found, let user pick (unless pick_first is true)
fn discover_package_type(package: &str, pick_first: bool) -> Result<(String, PackageType)> {
    // npm scoped packages start with @ - skip other checks
    if package.starts_with('@') {
        println!("{}", format!("Checking npm for '{}'...", package).cyan());
        let results = search_npm(package);
        if results.iter().any(|p| p.name == package) {
            println!("{}", format!("✓ Found in npm: {}", package).green());
            return Ok((package.to_string(), PackageType::Npm));
        }
        return Err(anyhow!("Package '{}' not found in npm", package));
    }

    // Search all registries to find exact matches
    println!(
        "{}",
        format!("Searching for '{}' across registries...", package).cyan()
    );

    let mut exact_matches: Vec<PackageInfo> = vec![];

    // Check cargo
    let cargo_results = search_cargo(package);
    if let Some(pkg) = cargo_results.iter().find(|p| p.name == package) {
        exact_matches.push(pkg.clone());
    }

    // Check PyPI
    let pypi_results = search_pypi(package);
    if let Some(pkg) = pypi_results.iter().find(|p| {
        p.name == package
            || p.name == package.replace('-', "_")
            || p.name == package.replace('_', "-")
    }) {
        exact_matches.push(pkg.clone());
    }

    // Check npm
    let npm_results = search_npm(package);
    if let Some(pkg) = npm_results.iter().find(|p| p.name == package) {
        exact_matches.push(pkg.clone());
    }

    // Sort by popularity (most downloads first)
    sort_by_popularity(&mut exact_matches);

    match exact_matches.len() {
        0 => Err(anyhow!(
            "Package '{}' not found in any registry (crates.io, PyPI, npm)",
            package
        )),
        1 => {
            let pkg = &exact_matches[0];
            println!(
                "{}",
                format!("✓ Found in {}: {}", pkg.registry.display_name(), pkg.name).green()
            );
            Ok((pkg.name.clone(), pkg.registry))
        }
        _ => {
            // Multiple matches found - already sorted by popularity
            if pick_first {
                // Auto-pick most popular (first after sort)
                let pkg = &exact_matches[0];
                println!(
                    "{}",
                    format!(
                        "✓ Multiple matches found, auto-selecting first: {} ({})",
                        pkg.name,
                        pkg.registry.display_name()
                    )
                    .green()
                );
                return Ok((pkg.name.clone(), pkg.registry));
            }

            // Let user pick
            println!();
            println!(
                "{}",
                format!(
                    "Found '{}' in {} registries. Please choose:",
                    package,
                    exact_matches.len()
                )
                .yellow()
                .bold()
            );
            println!();

            for (i, pkg) in exact_matches.iter().enumerate() {
                pkg.display(i);
            }

            print!(
                "{}",
                format!("Select a package (1-{}): ", exact_matches.len()).yellow()
            );
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let selection: usize = input.trim().parse().context("Invalid selection")?;

            if selection < 1 || selection > exact_matches.len() {
                return Err(anyhow!("Selection out of range"));
            }

            let selected = &exact_matches[selection - 1];
            println!();
            println!(
                "{}",
                format!(
                    "Selected: {} from {}",
                    selected.name,
                    selected.registry.display_name()
                )
                .green()
            );

            Ok((selected.name.clone(), selected.registry))
        }
    }
}

/// Get package type, using cache if available
fn get_package_type(package: &str, pick_first: bool) -> Result<(String, PackageType)> {
    let mut cache = PackageCache::load().unwrap_or_default();

    // Check cache first
    if let Some((pkg_name, pkg_type)) = cache.get(package) {
        println!(
            "{}",
            format!(
                "Using cached runtime for '{}': {} ({})",
                package,
                pkg_name,
                pkg_type.display_name()
            )
            .cyan()
        );
        return Ok((pkg_name, pkg_type));
    }

    // Discover package type
    let (pkg_name, pkg_type) = discover_package_type(package, pick_first)?;

    // Save to cache
    cache.set(package.to_string(), pkg_name.clone(), pkg_type);
    if let Err(e) = cache.save() {
        eprintln!(
            "{}",
            format!("Warning: Failed to save cache: {}", e).yellow()
        );
    }

    Ok((pkg_name, pkg_type))
}

/// Install uv if not present
fn install_uv() -> Result<()> {
    println!(
        "{}",
        "uv/uvx not found. Would you like to install it? [y/N]".yellow()
    );

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        return Err(anyhow!("Installation cancelled by user"));
    }

    println!("{}", "Installing uv...".cyan());

    let status = Command::new("sh")
        .args(["-c", "curl -LsSf https://astral.sh/uv/install.sh | sh"])
        .status()
        .context("Failed to install uv")?;

    if !status.success() {
        return Err(anyhow!("Failed to install uv"));
    }

    println!("{}", "✓ uv installed successfully".green());
    Ok(())
}

/// Run an MCP server package
fn run_package(package: &str, args: &[String], pick_first: bool) -> Result<()> {
    let (pkg_name, pkg_type) = get_package_type(package, pick_first)?;
    let runner = pkg_type.runner();

    // Check if runner exists
    if !command_exists(runner) {
        match pkg_type {
            PackageType::Python => {
                install_uv()?;
                if !command_exists(runner) {
                    return Err(anyhow!(
                        "{} still not found after installation. You may need to restart your shell or add it to PATH.",
                        runner
                    ));
                }
            }
            PackageType::Npm | PackageType::Cargo => {
                return Err(anyhow!(
                    "{} not found. {}",
                    runner,
                    pkg_type.install_instructions()
                ));
            }
        }
    }

    // Handle Cargo packages differently - install first, then run the binary
    if pkg_type == PackageType::Cargo {
        return run_cargo_package(&pkg_name, args);
    }

    println!(
        "{}",
        format!(
            "Running: {} {} {} {}",
            runner,
            if pkg_type == PackageType::Npm {
                "-y"
            } else {
                ""
            },
            pkg_name,
            args.join(" ")
        )
        .trim()
        .cyan()
    );

    let mut cmd = Command::new(runner);

    if pkg_type == PackageType::Npm {
        cmd.arg("-y");
    }

    cmd.arg(&pkg_name);
    cmd.args(args);

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().context(format!("Failed to spawn {}", runner))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        std::thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    println!("{}", line);
                }
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        std::thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("{}", line.red());
                }
            }
        });
    }

    let status = child.wait().context("Failed to wait for child process")?;

    if !status.success() {
        return Err(anyhow!("Process exited with status: {}", status));
    }

    Ok(())
}

/// Run a Cargo package by installing it first, then running the binary
fn run_cargo_package(package: &str, args: &[String]) -> Result<()> {
    if !command_exists(package) {
        println!(
            "{}",
            format!("Installing cargo package '{}'...", package).cyan()
        );

        let status = Command::new("cargo")
            .args(["install", package])
            .status()
            .context("Failed to run cargo install")?;

        if !status.success() {
            return Err(anyhow!("Failed to install cargo package: {}", package));
        }

        println!("{}", format!("✓ Installed {}", package).green());
    }

    println!(
        "{}",
        format!("Running: {} {}", package, args.join(" ")).cyan()
    );

    let mut cmd = Command::new(package);
    cmd.args(args);

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().context(format!("Failed to spawn {}", package))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        std::thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    println!("{}", line);
                }
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        std::thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("{}", line.red());
                }
            }
        });
    }

    let status = child.wait().context("Failed to wait for child process")?;

    if !status.success() {
        return Err(anyhow!("Process exited with status: {}", status));
    }

    Ok(())
}

/// Non-interactive search - just display results
fn search_package(query: &str) -> Result<()> {
    println!(
        "{}",
        format!("Searching for '{}' across all registries...", query).cyan()
    );
    println!();

    let mut all_packages = vec![];

    // Search cargo first
    print!("  Searching crates.io... ");
    std::io::stdout().flush()?;
    let cargo_results = search_cargo(query);
    println!("{} found", cargo_results.len());
    all_packages.extend(cargo_results);

    // Search PyPI
    print!("  Searching PyPI... ");
    std::io::stdout().flush()?;
    let pypi_results = search_pypi(query);
    println!("{} found", pypi_results.len());
    all_packages.extend(pypi_results);

    // Search npm
    print!("  Searching npm... ");
    std::io::stdout().flush()?;
    let npm_results = search_npm(query);
    println!("{} found", npm_results.len());
    all_packages.extend(npm_results);

    println!();

    if all_packages.is_empty() {
        println!(
            "{}",
            format!("No packages found matching '{}'", query).red()
        );
        return Ok(());
    }

    // Sort by popularity (most downloads first)
    sort_by_popularity(&mut all_packages);

    // Display all packages
    println!(
        "{}",
        format!("Found {} packages (sorted by popularity):", all_packages.len())
            .green()
            .bold()
    );
    println!();

    for (i, pkg) in all_packages.iter().enumerate() {
        pkg.display(i);
    }

    Ok(())
}

/// Interactive pick - show results and let user pick one to save to cache
fn pick_package(query: &str) -> Result<()> {
    let selection = search_and_select(query)?;

    if let Some((pkg_name, pkg_type)) = selection {
        // Ask if user wants to save to cache
        print!(
            "{}",
            format!(
                "Save '{}' -> {} ({}) to cache for future runs? [Y/n]: ",
                query,
                pkg_name,
                pkg_type.display_name()
            )
            .yellow()
        );
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() || input.eq_ignore_ascii_case("y") {
            let mut cache = PackageCache::load().unwrap_or_default();
            cache.set(query.to_string(), pkg_name.clone(), pkg_type);
            cache.save()?;
            println!("{}", "✓ Saved to cache".green());
        }

        // Ask if user wants to run it now
        print!("{}", "Run it now? [y/N]: ".yellow());
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("y") {
            run_package(&pkg_name, &[], false)?;
        }
    }

    Ok(())
}

/// Fetch the remote version from GitHub Cargo.toml
fn fetch_remote_version() -> Option<String> {
    let url = "https://raw.githubusercontent.com/xeb/eunice/master/Cargo.toml";
    let response = reqwest::blocking::get(url).ok()?;
    let text = response.text().ok()?;

    for line in text.lines() {
        if line.starts_with("version") {
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() >= 2 {
                return Some(parts[1].to_string());
            }
        }
    }
    None
}

/// Compare semantic versions, returns true if remote > local
fn is_newer_version(remote: &str, local: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    let remote_parts = parse(remote);
    let local_parts = parse(local);

    for i in 0..3 {
        let r = remote_parts.get(i).unwrap_or(&0);
        let l = local_parts.get(i).unwrap_or(&0);
        if r > l { return true; }
        if r < l { return false; }
    }
    false
}

/// Update mcpz (and eunice) from the git repository
/// Since mcpz is bundled with eunice, this updates both binaries
fn update_mcpz() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("{}", "Checking for updates...".cyan());
    println!("Current version: {}", current_version);

    // Check remote version
    if let Some(remote_version) = fetch_remote_version() {
        println!("Remote version:  {}", remote_version);
        println!();

        if !is_newer_version(&remote_version, current_version) {
            println!("{}", "Already up to date!".green());
            return Ok(());
        }

        println!("Update available: {} -> {}", current_version, remote_version);
    } else {
        println!("Could not fetch remote version, proceeding with update...");
    }
    println!();

    let status = Command::new("cargo")
        .env("CARGO_NET_GIT_FETCH_WITH_CLI", "true")
        .args(["install", "--git", "ssh://git@github.com/xeb/eunice.git", "--force"])
        .status()
        .context("Failed to run cargo install")?;

    if status.success() {
        println!("{}", "✓ Both eunice and mcpz updated successfully!".green());
        Ok(())
    } else {
        Err(anyhow!("Failed to update"))
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle --update flag
    if cli.update {
        return update_mcpz();
    }

    // Require a command if not updating
    let command = cli.command.ok_or_else(|| anyhow!("No command provided. Run 'mcpz --help' for usage."))?;

    match command {
        Commands::Run { package, first, args } => run_package(&package, &args, first),
        Commands::Search { package } => search_package(&package),
        Commands::Pick { package } => pick_package(&package),
        Commands::ClearCache => {
            PackageCache::clear()?;
            println!("{}", "✓ Cache cleared".green());
            Ok(())
        }
        Commands::Server { list, server_type } => {
            if list || server_type.is_none() {
                print_server_list();
                return Ok(());
            }
            match server_type.unwrap() {
                ServerType::Shell {
                    working_dir,
                    timeout,
                    shell,
                    allow,
                    deny,
                    no_stderr,
                    verbose,
                    http,
                    port,
                    host,
                    tls,
                    cert,
                    key,
                    origin,
                } => {
                    let shell_config = ShellServerConfig::new(
                        working_dir,
                        timeout,
                        shell,
                        allow,
                        deny,
                        no_stderr,
                        verbose,
                    );

                    if http {
                        // HTTP transport
                        use servers::shell::ShellServer;
                        let host_addr: IpAddr = host.parse()
                            .context("Invalid host address")?;
                        let http_config = http::HttpServerConfig::new(
                            port,
                            host_addr,
                            tls,
                            cert,
                            key,
                            origin,
                            verbose,
                        );
                        let server = ShellServer::new(shell_config);
                        let rt = tokio::runtime::Runtime::new()?;
                        rt.block_on(http::run_http_server(server, http_config))
                    } else {
                        // stdio transport
                        servers::run_shell_server(shell_config)
                    }
                }
                ServerType::Filesystem {
                    allowed_directories,
                    verbose,
                    http,
                    port,
                    host,
                    tls,
                    cert,
                    key,
                    origin,
                } => {
                    // Default to current directory if none specified
                    let dirs = if allowed_directories.is_empty() {
                        vec![std::env::current_dir()?]
                    } else {
                        allowed_directories
                    };
                    let fs_config = FilesystemServerConfig::new(dirs, verbose)?;

                    if http {
                        // HTTP transport
                        use servers::filesystem::FilesystemServer;
                        let host_addr: IpAddr = host.parse()
                            .context("Invalid host address")?;
                        let http_config = http::HttpServerConfig::new(
                            port,
                            host_addr,
                            tls,
                            cert,
                            key,
                            origin,
                            verbose,
                        );
                        let server = FilesystemServer::new(fs_config);
                        let rt = tokio::runtime::Runtime::new()?;
                        rt.block_on(http::run_http_server(server, http_config))
                    } else {
                        // stdio transport
                        servers::run_filesystem_server(fs_config)
                    }
                }
                ServerType::Sql {
                    connection,
                    readonly,
                    fullaccess: _,
                    timeout,
                    verbose,
                    http,
                    port,
                    host,
                    tls,
                    cert,
                    key,
                    origin,
                } => {
                    let access_mode = if readonly {
                        AccessMode::ReadOnly
                    } else {
                        AccessMode::FullAccess
                    };

                    // Detect database type from connection string
                    let db_type = DatabaseType::from_connection_string(&connection)
                        .context("Invalid connection string")?;

                    let sql_config = SqlServerConfig::new(connection.clone(), access_mode, timeout, verbose)
                        .context("Failed to create SQL server config")?;

                    if http {
                        // HTTP transport
                        use servers::sql::SqlServer;

                        // Connect using native driver
                        let rt = tokio::runtime::Runtime::new()?;
                        let pool = rt.block_on(connect_database(
                            &connection,
                            db_type,
                            std::time::Duration::from_secs(timeout),
                        )).context("Failed to connect to database")?;

                        let host_addr: IpAddr = host.parse()
                            .context("Invalid host address")?;
                        let http_config = http::HttpServerConfig::new(
                            port,
                            host_addr,
                            tls,
                            cert,
                            key,
                            origin,
                            verbose,
                        );

                        let server = SqlServer::new(sql_config, pool, rt);
                        let rt2 = tokio::runtime::Runtime::new()?;
                        rt2.block_on(http::run_http_server(server, http_config))
                    } else {
                        // stdio transport
                        servers::run_sql_server(sql_config)
                    }
                }
                ServerType::Browser {
                    cdp_port,
                    chrome_path,
                    timeout,
                    verbose,
                    http,
                    port,
                    host,
                    tls,
                    cert,
                    key,
                    origin,
                } => {
                    let browser_config = BrowserServerConfig::new(
                        cdp_port,
                        chrome_path,
                        timeout,
                        verbose,
                    ).with_http_mode(http);

                    if http {
                        // HTTP transport
                        use servers::browser::BrowserServer;
                        let host_addr: IpAddr = host.parse()
                            .context("Invalid host address")?;
                        let http_config = http::HttpServerConfig::new(
                            port,
                            host_addr,
                            tls,
                            cert,
                            key,
                            origin,
                            verbose,
                        );
                        let server = BrowserServer::new(browser_config);
                        let rt = tokio::runtime::Runtime::new()?;
                        rt.block_on(http::run_http_server(server, http_config))
                    } else {
                        // stdio transport
                        servers::run_browser_server(browser_config)
                    }
                }
            }
        }
        Commands::List => {
            print_full_list()?;
            Ok(())
        }
    }
}

/// Print list of available built-in MCP servers
fn print_server_list() {
    println!("{}", "Available built-in MCP servers:".green().bold());
    println!();
    println!("  {} - Execute shell commands", "shell".cyan());
    println!("    Usage: mcpz server shell [OPTIONS]");
    println!("    Server Options:");
    println!("      -w, --working-dir <PATH>  Working directory");
    println!("      -t, --timeout <SECONDS>   Command timeout (default: 30)");
    println!("      -s, --shell <PATH>        Shell to use (default: /bin/sh)");
    println!("      --allow <PATTERNS>        Allow only matching commands");
    println!("      --deny <PATTERNS>         Deny matching commands");
    println!("      --no-stderr               Suppress stderr in output");
    println!("      -v, --verbose             Enable debug logging");
    println!();
    println!("  {} - Filesystem operations", "filesystem".cyan());
    println!("    Usage: mcpz server filesystem [OPTIONS]");
    println!("    Server Options:");
    println!("      -d, --dir <PATH>          Allowed directory (default: current dir, can repeat)");
    println!("      -v, --verbose             Enable debug logging");
    println!();
    println!("  {} - SQL database queries", "sql".cyan());
    println!("    Usage: mcpz server sql --connection <URL> --readonly|--fullaccess");
    println!("    Server Options:");
    println!("      -c, --connection <URL>    Database connection string (required)");
    println!("      --readonly                Only allow SELECT queries");
    println!("      --fullaccess              Allow all SQL statements");
    println!("      -t, --timeout <SECONDS>   Query timeout (default: 30)");
    println!("      -v, --verbose             Enable debug logging");
    println!("    Supported databases: PostgreSQL, MySQL, MariaDB, SQLite");
    println!();
    println!("  {} - Browser automation via Chrome DevTools", "browser".cyan());
    println!("    Usage: mcpz server browser [OPTIONS]");
    println!("    Server Options:");
    println!("      -p, --cdp-port <PORT>     Chrome DevTools port (default: 9222)");
    println!("      --chrome-path <PATH>      Path to Chrome executable");
    println!("      -t, --timeout <SECONDS>   Page load timeout (default: 30)");
    println!("      -v, --verbose             Enable debug logging");
    println!("    Tools: start_browser, stop_browser, open_url, get_page,");
    println!("           get_page_as_markdown, get_screenshot, execute_script");
    println!();
    println!("{}", "HTTP Transport Options (add to any server):".yellow().bold());
    println!("      --http                    Use HTTP transport instead of stdio");
    println!("      -p, --port <PORT>         HTTP port (default: 3000)");
    println!("      -H, --host <HOST>         Bind address (default: 127.0.0.1)");
    println!("      --tls                     Enable HTTPS (auto-generates self-signed cert)");
    println!("      --cert <PATH>             TLS certificate path (use with --key)");
    println!("      --key <PATH>              TLS private key path (use with --cert)");
    println!("      --origin <ORIGINS>        Allowed CORS origins (comma-separated)");
    println!();
    println!("{}", "Examples:".green());
    println!("  mcpz server shell                         # stdio transport");
    println!("  mcpz server shell --http                  # HTTP on localhost:3000");
    println!("  mcpz server filesystem --http --tls       # HTTPS with self-signed cert");
    println!("  mcpz server shell --http -p 8080 --tls    # HTTPS on port 8080");
    println!();
    println!("{}", "SQL Examples:".green());
    println!("  mcpz server sql -c postgres://user:pass@localhost/db --readonly");
    println!("  mcpz server sql -c mysql://user:pass@localhost/db --fullaccess");
    println!("  mcpz server sql -c sqlite:///path/to/file.db --readonly");
    println!("  mcpz server sql -c sqlite::memory: --fullaccess");
    println!();
    println!("{}", "Browser Examples:".green());
    println!("  mcpz server browser                       # Start browser server");
    println!("  mcpz server browser -p 9333               # Custom DevTools port");
    println!("  mcpz server browser --http --port 3000    # HTTP transport");
    println!();
    println!("Run 'mcpz server <SERVER> --help' for more details.");
}

/// Print full list of cached packages and available servers
fn print_full_list() -> Result<()> {
    // Print cached package mappings
    println!("{}", "Cached package mappings:".green().bold());
    println!();

    let cache = PackageCache::load().unwrap_or_default();
    if cache.packages.is_empty() {
        println!("  (no cached packages)");
    } else {
        let mut entries: Vec<_> = cache.packages.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (search_term, (package_name, pkg_type)) in entries {
            println!(
                "  {} -> {} ({})",
                search_term.cyan(),
                package_name,
                pkg_type.display_name()
            );
            println!(
                "    Run: {}",
                format!("mcpz run {}", search_term).yellow()
            );
        }
    }

    println!();
    println!("{}", "Built-in MCP servers:".green().bold());
    println!();
    println!("  {} - Execute shell commands", "shell".cyan());
    println!("    Run: {}", "mcpz server shell".yellow());
    println!();
    println!("  {} - Filesystem operations", "filesystem".cyan());
    println!("    Run: {}", "mcpz server filesystem".yellow());
    println!();
    println!("  {} - SQL database queries", "sql".cyan());
    println!("    Run: {}", "mcpz server sql -c <connection> --readonly".yellow());
    println!();
    println!("  {} - Browser automation", "browser".cyan());
    println!("    Run: {}", "mcpz server browser".yellow());
    println!();
    println!("Use 'mcpz server --list' for detailed server options.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_type_runner() {
        assert_eq!(PackageType::Npm.runner(), "npx");
        assert_eq!(PackageType::Python.runner(), "uvx");
        assert_eq!(PackageType::Cargo.runner(), "cargo");
    }

    #[test]
    fn test_package_type_display_name() {
        assert_eq!(PackageType::Npm.display_name(), "npm");
        assert_eq!(PackageType::Python.display_name(), "PyPI");
        assert_eq!(PackageType::Cargo.display_name(), "crates.io");
    }

    #[test]
    fn test_command_exists_which() {
        assert!(command_exists("which"));
    }

    #[test]
    fn test_command_exists_nonexistent() {
        assert!(!command_exists("this-command-definitely-does-not-exist-12345"));
    }

    #[test]
    fn test_cli_parse_run() {
        let cli = Cli::parse_from(["mcpz", "run", "@modelcontextprotocol/server-filesystem", "."]);
        match cli.command.unwrap() {
            Commands::Run { package, first, args } => {
                assert_eq!(package, "@modelcontextprotocol/server-filesystem");
                assert!(!first);
                assert_eq!(args, vec!["."]);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_parse_run_no_args() {
        let cli = Cli::parse_from(["mcpz", "run", "mcp-server-time"]);
        match cli.command.unwrap() {
            Commands::Run { package, first, args } => {
                assert_eq!(package, "mcp-server-time");
                assert!(!first);
                assert!(args.is_empty());
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_parse_run_first() {
        let cli = Cli::parse_from(["mcpz", "run", "--first", "mcp-server-time"]);
        match cli.command.unwrap() {
            Commands::Run { package, first, args } => {
                assert_eq!(package, "mcp-server-time");
                assert!(first);
                assert!(args.is_empty());
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_parse_search() {
        let cli = Cli::parse_from(["mcpz", "search", "mcp-server-time"]);
        match cli.command.unwrap() {
            Commands::Search { package } => {
                assert_eq!(package, "mcp-server-time");
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_cli_parse_pick() {
        let cli = Cli::parse_from(["mcpz", "pick", "mcp-server-time"]);
        match cli.command.unwrap() {
            Commands::Pick { package } => {
                assert_eq!(package, "mcp-server-time");
            }
            _ => panic!("Expected Pick command"),
        }
    }

    #[test]
    fn test_cli_parse_clear_cache() {
        let cli = Cli::parse_from(["mcpz", "clear-cache"]);
        assert!(matches!(cli.command, Some(Commands::ClearCache)));
    }

    #[test]
    fn test_package_type_install_instructions() {
        let npm_instructions = PackageType::Npm.install_instructions();
        assert!(npm_instructions.contains("nodejs") || npm_instructions.contains("Node"));

        let python_instructions = PackageType::Python.install_instructions();
        assert!(python_instructions.contains("astral.sh"));

        let cargo_instructions = PackageType::Cargo.install_instructions();
        assert!(cargo_instructions.contains("rustup") || cargo_instructions.contains("Rust"));
    }

    #[test]
    fn test_cache_serialization() {
        let mut cache = PackageCache::default();
        cache.set(
            "test-search".to_string(),
            "actual-package".to_string(),
            PackageType::Python,
        );
        cache.set(
            "another".to_string(),
            "another-pkg".to_string(),
            PackageType::Npm,
        );

        let serialized = toml::to_string(&cache).unwrap();
        let deserialized: PackageCache = toml::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized.get("test-search"),
            Some(("actual-package".to_string(), PackageType::Python))
        );
        assert_eq!(
            deserialized.get("another"),
            Some(("another-pkg".to_string(), PackageType::Npm))
        );
    }

    // Shell server tests

    #[test]
    fn test_cli_parse_server_list_flag() {
        let cli = Cli::parse_from(["mcpz", "server", "--list"]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(list);
                assert!(server_type.is_none());
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_no_subcommand() {
        let cli = Cli::parse_from(["mcpz", "server"]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                assert!(server_type.is_none());
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_shell() {
        let cli = Cli::parse_from(["mcpz", "server", "shell"]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Shell { working_dir, timeout, shell, allow, deny, no_stderr, verbose, http, .. }) => {
                        assert!(working_dir.is_none());
                        assert_eq!(timeout, 30);
                        assert_eq!(shell, "/bin/sh");
                        assert!(allow.is_none());
                        assert!(deny.is_none());
                        assert!(!no_stderr);
                        assert!(!verbose);
                        assert!(!http);
                    }
                    _ => panic!("Expected Shell server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_shell_with_options() {
        let cli = Cli::parse_from([
            "mcpz", "server", "shell",
            "--working-dir", "/tmp",
            "--timeout", "60",
            "--shell", "/bin/bash",
            "--allow", "ls*,cat*",
            "--deny", "rm*,sudo*",
            "--no-stderr",
            "--verbose",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Shell { working_dir, timeout, shell, allow, deny, no_stderr, verbose, .. }) => {
                        assert_eq!(working_dir, Some(PathBuf::from("/tmp")));
                        assert_eq!(timeout, 60);
                        assert_eq!(shell, "/bin/bash");
                        assert_eq!(allow, Some("ls*,cat*".to_string()));
                        assert_eq!(deny, Some("rm*,sudo*".to_string()));
                        assert!(no_stderr);
                        assert!(verbose);
                    }
                    _ => panic!("Expected Shell server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_shell_with_http() {
        let cli = Cli::parse_from([
            "mcpz", "server", "shell",
            "--http",
            "-p", "8080",
            "-H", "0.0.0.0",
            "--tls",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Shell { http, port, host, tls, cert, key, .. }) => {
                        assert!(http);
                        assert_eq!(port, 8080);
                        assert_eq!(host, "0.0.0.0");
                        assert!(tls);
                        assert!(cert.is_none());
                        assert!(key.is_none());
                    }
                    _ => panic!("Expected Shell server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_filesystem() {
        let cli = Cli::parse_from(["mcpz", "server", "filesystem", "-d", "/tmp"]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Filesystem { allowed_directories, verbose, http, .. }) => {
                        assert_eq!(allowed_directories, vec![PathBuf::from("/tmp")]);
                        assert!(!verbose);
                        assert!(!http);
                    }
                    _ => panic!("Expected Filesystem server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_filesystem_default_dir() {
        let cli = Cli::parse_from(["mcpz", "server", "filesystem"]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Filesystem { allowed_directories, verbose, .. }) => {
                        // No directories specified - will default to cwd at runtime
                        assert!(allowed_directories.is_empty());
                        assert!(!verbose);
                    }
                    _ => panic!("Expected Filesystem server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_filesystem_multiple_dirs() {
        let cli = Cli::parse_from([
            "mcpz", "server", "filesystem",
            "-d", "/tmp",
            "-d", "/home",
            "--verbose",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Filesystem { allowed_directories, verbose, .. }) => {
                        assert_eq!(allowed_directories, vec![PathBuf::from("/tmp"), PathBuf::from("/home")]);
                        assert!(verbose);
                    }
                    _ => panic!("Expected Filesystem server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_filesystem_with_http() {
        let cli = Cli::parse_from([
            "mcpz", "server", "filesystem",
            "-d", "/data",
            "--http",
            "-p", "9000",
            "--tls",
            "--cert", "/path/to/cert.pem",
            "--key", "/path/to/key.pem",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Filesystem { allowed_directories, http, port, tls, cert, key, .. }) => {
                        assert_eq!(allowed_directories, vec![PathBuf::from("/data")]);
                        assert!(http);
                        assert_eq!(port, 9000);
                        assert!(tls);
                        assert_eq!(cert, Some(PathBuf::from("/path/to/cert.pem")));
                        assert_eq!(key, Some(PathBuf::from("/path/to/key.pem")));
                    }
                    _ => panic!("Expected Filesystem server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_list() {
        let cli = Cli::parse_from(["mcpz", "list"]);
        assert!(matches!(cli.command, Some(Commands::List)));
    }

    #[test]
    fn test_print_server_list_does_not_panic() {
        // Just verify it doesn't panic
        print_server_list();
    }

    #[test]
    fn test_print_full_list_does_not_panic() {
        // Just verify it doesn't panic (uses actual cache file if present)
        let result = print_full_list();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_server_short_list_flag() {
        let cli = Cli::parse_from(["mcpz", "server", "-l"]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(list);
                assert!(server_type.is_none());
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_sql_readonly() {
        let cli = Cli::parse_from([
            "mcpz", "server", "sql",
            "--connection", "postgres://user:pass@localhost:5432/mydb",
            "--readonly",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Sql { connection, readonly, fullaccess, timeout, verbose, http, .. }) => {
                        assert_eq!(connection, "postgres://user:pass@localhost:5432/mydb");
                        assert!(readonly);
                        assert!(!fullaccess);
                        assert_eq!(timeout, 30);
                        assert!(!verbose);
                        assert!(!http);
                    }
                    _ => panic!("Expected Sql server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_sql_fullaccess() {
        let cli = Cli::parse_from([
            "mcpz", "server", "sql",
            "-c", "mysql://root:secret@localhost:3306/production",
            "--fullaccess",
            "--verbose",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Sql { connection, readonly, fullaccess, verbose, .. }) => {
                        assert_eq!(connection, "mysql://root:secret@localhost:3306/production");
                        assert!(!readonly);
                        assert!(fullaccess);
                        assert!(verbose);
                    }
                    _ => panic!("Expected Sql server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_sql_sqlite() {
        let cli = Cli::parse_from([
            "mcpz", "server", "sql",
            "-c", "sqlite:///tmp/test.db",
            "--readonly",
            "-t", "60",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Sql { connection, readonly, timeout, .. }) => {
                        assert_eq!(connection, "sqlite:///tmp/test.db");
                        assert!(readonly);
                        assert_eq!(timeout, 60);
                    }
                    _ => panic!("Expected Sql server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_sql_sqlite_memory() {
        let cli = Cli::parse_from([
            "mcpz", "server", "sql",
            "-c", "sqlite::memory:",
            "--fullaccess",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Sql { connection, fullaccess, .. }) => {
                        assert_eq!(connection, "sqlite::memory:");
                        assert!(fullaccess);
                    }
                    _ => panic!("Expected Sql server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_parse_server_sql_with_http() {
        let cli = Cli::parse_from([
            "mcpz", "server", "sql",
            "-c", "postgres://localhost/db",
            "--readonly",
            "--http",
            "-p", "8080",
            "--tls",
        ]);
        match cli.command.unwrap() {
            Commands::Server { list, server_type } => {
                assert!(!list);
                match server_type {
                    Some(ServerType::Sql { connection, readonly, http, port, tls, .. }) => {
                        assert_eq!(connection, "postgres://localhost/db");
                        assert!(readonly);
                        assert!(http);
                        assert_eq!(port, 8080);
                        assert!(tls);
                    }
                    _ => panic!("Expected Sql server type"),
                }
            }
            _ => panic!("Expected Server command"),
        }
    }
}
