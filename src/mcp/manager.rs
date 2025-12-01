use crate::mcp::http_server::HttpMcpServer;
use crate::mcp::server::{McpServer, SpawnedServer};
use crate::models::{McpConfig, Tool};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::task::JoinHandle;

/// A ready MCP server (either stdio or HTTP)
pub enum ReadyServer {
    Stdio(McpServer),
    Http(HttpMcpServer),
}

impl ReadyServer {
    pub fn tools(&self) -> &[Tool] {
        match self {
            ReadyServer::Stdio(s) => &s.tools,
            ReadyServer::Http(s) => &s.tools,
        }
    }

    pub async fn call_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<String> {
        match self {
            ReadyServer::Stdio(s) => s.call_tool(tool_name, arguments).await,
            ReadyServer::Http(s) => s.call_tool(tool_name, arguments).await,
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        match self {
            ReadyServer::Stdio(s) => s.stop().await,
            ReadyServer::Http(_) => Ok(()), // HTTP servers don't need stopping
        }
    }
}

/// State of an MCP server during lazy initialization
pub enum ServerState {
    /// Server is starting up in background
    Initializing(JoinHandle<Result<ReadyServer>>),
    /// Server is ready to use
    Ready(ReadyServer),
    /// Server failed to start
    Failed(String),
}

/// Manages multiple MCP servers and routes tool calls with lazy loading
pub struct McpManager {
    servers: HashMap<String, ServerState>,
    /// Whether to print status messages
    silent: bool,
    /// Whether to print verbose debug messages
    verbose: bool,
    /// Optional list of allowed tool patterns (if empty, all tools are allowed)
    allowed_tools: Vec<String>,
    /// Optional list of denied tool patterns (tools matching these are excluded)
    denied_tools: Vec<String>,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            silent: false,
            verbose: false,
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        }
    }

    /// Set the allowed tools filter
    pub fn set_allowed_tools(&mut self, allowed: Vec<String>) {
        self.allowed_tools = allowed;
    }

    /// Set the denied tools filter
    pub fn set_denied_tools(&mut self, denied: Vec<String>) {
        self.denied_tools = denied;
    }

    /// Start all MCP servers from configuration in the background (non-blocking)
    /// Returns immediately - servers initialize in parallel background tasks
    pub fn start_servers_background(&mut self, config: &McpConfig, silent: bool, verbose: bool) {
        self.silent = silent;
        self.verbose = verbose;

        for (name, server_config) in &config.mcp_servers {
            // Check if this is an HTTP server or stdio server
            if server_config.is_http() {
                self.start_http_server(name, server_config.url.as_ref().unwrap(), server_config.timeout, silent, verbose);
            } else {
                self.start_stdio_server(name, &server_config.command, &server_config.args, server_config.timeout, silent, verbose);
            }
        }
    }

    /// Start an HTTP-based MCP server in the background
    fn start_http_server(&mut self, name: &str, url: &str, timeout: Option<u64>, silent: bool, verbose: bool) {
        if !silent {
            eprintln!("Connecting to HTTP MCP server: {} at {}", name, url);
        }

        if verbose {
            eprintln!("  [verbose] HTTP MCP server '{}' config:", name);
            eprintln!("    url: '{}'", url);
            if let Some(t) = timeout {
                eprintln!("    timeout: {}s", t);
            }
        }

        let server_name = name.to_string();
        let server_url = url.to_string();
        let server_timeout = timeout;
        let is_silent = silent;
        let is_verbose = verbose;

        let handle = tokio::spawn(async move {
            if is_verbose {
                eprintln!("  [verbose] Connecting to HTTP MCP server '{}'...", server_name);
            }
            let result = HttpMcpServer::connect(&server_name, &server_url, server_timeout, is_verbose).await;

            if !is_silent {
                match &result {
                    Ok(server) => {
                        eprintln!(
                            "  {} ready (HTTP): {} tools",
                            server_name,
                            server.tools.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("  {} failed (HTTP): {}", server_name, e);
                    }
                }
            }

            result.map(ReadyServer::Http)
        });

        self.servers
            .insert(name.to_string(), ServerState::Initializing(handle));
    }

    /// Start a stdio-based MCP server in the background
    fn start_stdio_server(&mut self, name: &str, command: &str, args: &[String], timeout: Option<u64>, silent: bool, verbose: bool) {
        if !silent {
            eprintln!("Starting MCP server: {} (background)", name);
        }

        if verbose {
            eprintln!("  [verbose] MCP server '{}' spawn config:", name);
            eprintln!("    command: '{}'", command);
            eprintln!("    args: {:?}", args);
            if let Some(t) = timeout {
                eprintln!("    timeout: {}s", t);
            }
        }

        // Spawn the process synchronously (fast, doesn't block)
        match SpawnedServer::spawn(name, command, args, timeout, verbose) {
            Ok(spawned) => {
                let server_name = name.to_string();
                let is_silent = silent;
                let is_verbose = verbose;

                // Spawn async initialization in background
                let handle = tokio::spawn(async move {
                    let result = spawned.initialize(is_verbose).await;
                    if !is_silent {
                        match &result {
                            Ok(server) => {
                                eprintln!(
                                    "  {} ready: {} tools",
                                    server_name,
                                    server.tools.len()
                                );
                            }
                            Err(e) => {
                                eprintln!("  {} failed: {}", server_name, e);
                            }
                        }
                    }
                    result.map(ReadyServer::Stdio)
                });

                self.servers
                    .insert(name.to_string(), ServerState::Initializing(handle));
            }
            Err(e) => {
                if !silent {
                    eprintln!("Failed to spawn MCP server '{}': {}", name, e);
                }
                self.servers
                    .insert(name.to_string(), ServerState::Failed(e.to_string()));
            }
        }
    }

    /// Wait for all pending servers to finish initializing
    pub async fn await_all_servers(&mut self) {
        let pending_names: Vec<String> = self
            .servers
            .iter()
            .filter_map(|(name, state)| {
                if matches!(state, ServerState::Initializing(_)) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for name in pending_names {
            self.await_server(&name).await;
        }
    }

    /// Await a specific server if it's still initializing
    async fn await_server(&mut self, name: &str) {
        if let Some(state) = self.servers.remove(name) {
            match state {
                ServerState::Initializing(handle) => {
                    match handle.await {
                        Ok(Ok(server)) => {
                            self.servers
                                .insert(name.to_string(), ServerState::Ready(server));
                        }
                        Ok(Err(e)) => {
                            self.servers
                                .insert(name.to_string(), ServerState::Failed(e.to_string()));
                        }
                        Err(e) => {
                            self.servers.insert(
                                name.to_string(),
                                ServerState::Failed(format!("Task panic: {}", e)),
                            );
                        }
                    }
                }
                // Put back if already in final state
                other => {
                    self.servers.insert(name.to_string(), other);
                }
            }
        }
    }

    /// Get all available tools from ready servers
    /// Note: This only returns tools from servers that have finished initializing
    /// Applies allowed_tools filter (whitelist) and denied_tools filter (blacklist)
    pub fn get_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();
        for state in self.servers.values() {
            if let ServerState::Ready(server) = state {
                tools.extend(server.tools().iter().cloned());
            }
        }

        // Filter by allowed_tools patterns if set (whitelist)
        if !self.allowed_tools.is_empty() {
            tools.retain(|t| {
                self.allowed_tools.iter().any(|pattern| {
                    crate::mcp::tool_matches_pattern(&t.function.name, pattern)
                })
            });
        }

        // Filter out denied_tools patterns (blacklist)
        if !self.denied_tools.is_empty() {
            tools.retain(|t| {
                !self.denied_tools.iter().any(|pattern| {
                    crate::mcp::tool_matches_pattern(&t.function.name, pattern)
                })
            });
        }

        tools
    }

    /// Execute a tool call by routing to the appropriate server
    /// Will await the server if it's still initializing
    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String> {
        // Find the server that owns this tool by checking server name prefix
        let best_match = self
            .servers
            .keys()
            .filter_map(|server_name| {
                let full_prefix = format!("{}_", server_name);
                tool_name
                    .strip_prefix(&full_prefix)
                    .map(|actual_tool_name| (server_name.clone(), actual_tool_name.to_string()))
            })
            .next();

        let Some((server_name, actual_tool_name)) = best_match else {
            return Err(anyhow!("No server found for tool '{}'", tool_name));
        };

        // Await the server if it's still initializing
        self.await_server(&server_name).await;

        // Now get the server and execute
        let state = self
            .servers
            .get_mut(&server_name)
            .ok_or_else(|| anyhow!("Server '{}' not found", server_name))?;

        match state {
            ServerState::Ready(server) => server.call_tool(&actual_tool_name, arguments).await,
            ServerState::Failed(err) => Err(anyhow!(
                "Server '{}' failed to start: {}",
                server_name,
                err
            )),
            ServerState::Initializing(_) => {
                // Should not happen after await_server
                Err(anyhow!("Server '{}' still initializing (unexpected)", server_name))
            }
        }
    }

    /// Get server information for display
    pub fn get_server_info(&self) -> Vec<(String, usize, Vec<String>)> {
        let mut info = Vec::new();
        for (name, state) in &self.servers {
            match state {
                ServerState::Ready(server) => {
                    let tools = server.tools();
                    let tool_names: Vec<String> = tools
                        .iter()
                        .map(|t| t.function.name.clone())
                        .collect();
                    info.push((name.clone(), tools.len(), tool_names));
                }
                ServerState::Initializing(_) => {
                    info.push((name.clone(), 0, vec!["(starting...)".to_string()]));
                }
                ServerState::Failed(err) => {
                    info.push((name.clone(), 0, vec![format!("(failed: {})", err)]));
                }
            }
        }
        info.sort_by(|a, b| a.0.cmp(&b.0));
        info
    }

    /// Shutdown all MCP servers
    pub async fn shutdown(&mut self) -> Result<()> {
        // First, wait for any initializing servers
        self.await_all_servers().await;

        let server_names: Vec<String> = self.servers.keys().cloned().collect();

        for name in server_names {
            if let Some(ServerState::Ready(mut server)) = self.servers.remove(&name) {
                if let Err(e) = server.stop().await {
                    eprintln!("Error stopping MCP server '{}': {}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Get information about failed servers for error reporting to the model
    pub fn get_failed_servers(&self) -> Vec<(String, String)> {
        self.servers
            .iter()
            .filter_map(|(name, state)| {
                if let ServerState::Failed(err) = state {
                    Some((name.clone(), err.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if any servers are loaded (in any state)
    #[allow(dead_code)]
    pub fn has_servers(&self) -> bool {
        !self.servers.is_empty()
    }

    /// Get number of servers
    #[allow(dead_code)]
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Get number of ready servers
    #[allow(dead_code)]
    pub fn ready_count(&self) -> usize {
        self.servers
            .values()
            .filter(|s| matches!(s, ServerState::Ready(_)))
            .count()
    }

    /// Get number of servers still initializing
    #[allow(dead_code)]
    pub fn pending_count(&self) -> usize {
        self.servers
            .values()
            .filter(|s| matches!(s, ServerState::Initializing(_)))
            .count()
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager() {
        let manager = McpManager::new();
        assert!(!manager.has_servers());
        assert_eq!(manager.server_count(), 0);
        assert_eq!(manager.ready_count(), 0);
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_get_tools_empty() {
        let manager = McpManager::new();
        let tools = manager.get_tools();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_get_server_info_empty() {
        let manager = McpManager::new();
        let info = manager.get_server_info();
        assert!(info.is_empty());
    }

    #[tokio::test]
    async fn test_failed_server_state() {
        let mut manager = McpManager::new();
        manager.servers.insert(
            "test_server".to_string(),
            ServerState::Failed("spawn error".to_string()),
        );

        assert_eq!(manager.server_count(), 1);
        assert_eq!(manager.ready_count(), 0);

        // Tool execution should fail with descriptive error
        let result = manager
            .execute_tool("test_server_sometool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed to start"));
    }

    #[test]
    fn test_get_server_info_with_failed() {
        let mut manager = McpManager::new();
        manager.servers.insert(
            "failed_server".to_string(),
            ServerState::Failed("connection refused".to_string()),
        );

        let info = manager.get_server_info();
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].0, "failed_server");
        assert_eq!(info[0].1, 0);
        assert!(info[0].2[0].contains("failed"));
    }
}
