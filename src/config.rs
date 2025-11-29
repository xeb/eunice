use crate::models::{McpConfig, McpServerConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Check if mcpz is installed
fn has_mcpz() -> bool {
    std::process::Command::new("mcpz").arg("--version").output().is_ok()
}

/// Embedded DMN (Default Mode Network) MCP configuration
/// Minimal set: shell + filesystem (interpret_image is built-in)
pub fn get_dmn_mcp_config() -> McpConfig {
    let mut servers = HashMap::new();
    let use_mcpz = has_mcpz();

    servers.insert(
        "shell".to_string(),
        if use_mcpz {
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "shell".into()], url: None }
        } else {
            McpServerConfig { command: "uvx".into(), args: vec!["git+https://github.com/emsi/mcp-server-shell".into()], url: None }
        },
    );

    servers.insert(
        "filesystem".to_string(),
        if use_mcpz {
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "filesystem".into()], url: None }
        } else {
            McpServerConfig { command: "npx".into(), args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), ".".into()], url: None }
        },
    );

    McpConfig {
        mcp_servers: servers,
        agents: HashMap::new(),
    }
}

/// Load MCP configuration from a JSON or TOML file
pub fn load_mcp_config(path: &Path) -> Result<McpConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    // Determine format based on file extension
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "toml" => toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config file: {}", path.display())),
        _ => serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON config file: {}", path.display())),
    }
}

/// DMN (Default Mode Network) instructions loaded from external file
pub const DMN_INSTRUCTIONS: &str = include_str!("../dmn_instructions.md");

/// LLM context files embedded for --llms-txt and --llms-full-txt flags
pub const LLMS_TXT: &str = include_str!("../llms.txt");
pub const LLMS_FULL_TXT: &str = include_str!("../llms-full.txt");
