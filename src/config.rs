use crate::models::{McpConfig, McpServerConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Check if mcpz is installed
fn has_mcpz() -> bool {
    std::process::Command::new("mcpz").arg("--version").output().is_ok()
}

/// Embedded DMN (Default Mode Network) MCP configuration
pub fn get_dmn_mcp_config() -> McpConfig {
    let mut servers = HashMap::new();
    let use_mcpz = has_mcpz();

    servers.insert(
        "shell".to_string(),
        if use_mcpz {
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "shell".into()] }
        } else {
            McpServerConfig { command: "uvx".into(), args: vec!["git+https://github.com/emsi/mcp-server-shell".into()] }
        },
    );

    servers.insert(
        "filesystem".to_string(),
        if use_mcpz {
            McpServerConfig { command: "mcpz".into(), args: vec!["server".into(), "filesystem".into()] }
        } else {
            McpServerConfig { command: "npx".into(), args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), ".".into()] }
        },
    );

    servers.insert(
        "text-editor".to_string(),
        McpServerConfig {
            command: "uvx".to_string(),
            args: vec!["mcp-text-editor".to_string()],
        },
    );

    servers.insert(
        "grep".to_string(),
        McpServerConfig {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "mcp-ripgrep@latest".to_string()],
        },
    );

    servers.insert(
        "memory".to_string(),
        McpServerConfig {
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-memory".to_string(),
                "~/.eunice".to_string(),
            ],
        },
    );

    servers.insert(
        "web".to_string(),
        McpServerConfig {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@brave/brave-search-mcp-server".to_string()],
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
