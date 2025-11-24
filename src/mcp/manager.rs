use crate::mcp::server::McpServer;
use crate::models::{McpConfig, Tool};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Manages multiple MCP servers and routes tool calls
pub struct McpManager {
    servers: HashMap<String, McpServer>,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Load and start all MCP servers from configuration
    pub async fn load_config(&mut self, config: &McpConfig, silent: bool) -> Result<()> {
        for (name, server_config) in &config.mcp_servers {
            if !silent {
                eprintln!("Starting MCP server: {}", name);
            }

            match McpServer::start(name, &server_config.command, &server_config.args).await {
                Ok(server) => {
                    if !silent {
                        eprintln!(
                            "  {} tools discovered: {}",
                            server.tools.len(),
                            server
                                .tools
                                .iter()
                                .map(|t| t.function.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                    self.servers.insert(name.clone(), server);
                }
                Err(e) => {
                    eprintln!("Failed to start MCP server '{}': {}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Get all available tools from all servers
    pub fn get_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();
        for server in self.servers.values() {
            tools.extend(server.tools.clone());
        }
        tools
    }

    /// Execute a tool call by routing to the appropriate server
    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String> {
        // Find the server that owns this tool
        // Use longest prefix match to handle server names with underscores
        let mut best_match: Option<(String, String)> = None;

        for server_name in self.servers.keys() {
            let prefix = format!("{}_", server_name);
            if tool_name.starts_with(&prefix) {
                if best_match.is_none() || server_name.len() > best_match.as_ref().unwrap().0.len() {
                    let actual_tool_name = tool_name[prefix.len()..].to_string();
                    best_match = Some((server_name.clone(), actual_tool_name));
                }
            }
        }

        if let Some((server_name, actual_tool_name)) = best_match {
            let server = self
                .servers
                .get_mut(&server_name)
                .ok_or_else(|| anyhow!("Server '{}' not found", server_name))?;

            server.call_tool(&actual_tool_name, arguments).await
        } else {
            Err(anyhow!("No server found for tool '{}'", tool_name))
        }
    }

    /// Get server information for display
    pub fn get_server_info(&self) -> Vec<(String, usize, Vec<String>)> {
        let mut info = Vec::new();
        for (name, server) in &self.servers {
            let tool_names: Vec<String> = server
                .tools
                .iter()
                .map(|t| t.function.name.clone())
                .collect();
            info.push((name.clone(), server.tools.len(), tool_names));
        }
        info.sort_by(|a, b| a.0.cmp(&b.0));
        info
    }

    /// Shutdown all MCP servers
    pub async fn shutdown(&mut self) -> Result<()> {
        let server_names: Vec<String> = self.servers.keys().cloned().collect();

        for name in server_names {
            if let Some(mut server) = self.servers.remove(&name) {
                if let Err(e) = server.stop().await {
                    eprintln!("Error stopping MCP server '{}': {}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Check if any servers are loaded
    #[allow(dead_code)]
    pub fn has_servers(&self) -> bool {
        !self.servers.is_empty()
    }

    /// Get number of servers
    #[allow(dead_code)]
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
