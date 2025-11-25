use crate::models::{McpConfig, McpServerConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Embedded DMN (Default Mode Network) MCP configuration
pub fn get_dmn_mcp_config() -> McpConfig {
    let mut servers = HashMap::new();

    servers.insert(
        "shell".to_string(),
        McpServerConfig {
            command: "uvx".to_string(),
            args: vec!["git+https://github.com/emsi/mcp-server-shell".to_string()],
        },
    );

    servers.insert(
        "filesystem".to_string(),
        McpServerConfig {
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                ".".to_string(),
            ],
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
    }
}

/// Load MCP configuration from a JSON file
pub fn load_mcp_config(path: &Path) -> Result<McpConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))
}

/// DMN (Default Mode Network) instructions loaded from external file
pub const DMN_INSTRUCTIONS: &str = include_str!("../dmn_instructions.md");
