use crate::mcp::{sanitize_schema, sanitize_tool_name};
use crate::models::{
    FunctionSpec, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    McpToolResult, McpToolsResult, Tool,
};
use anyhow::{anyhow, Context, Result};
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::time::{timeout, Duration};

/// Default timeout in seconds for MCP requests (10 minutes)
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Represents a spawned but not yet initialized MCP server
pub struct SpawnedServer {
    name: String,
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    timeout_secs: u64,
}

impl SpawnedServer {
    /// Spawn a server process without initializing it
    /// Note: This is synchronous but fast - it just spawns the process
    pub fn spawn(name: &str, command: &str, args: &[String], timeout_secs: Option<u64>, verbose: bool) -> Result<Self> {
        // Validate command is not empty
        if command.is_empty() {
            return Err(anyhow!(
                "MCP server '{}' has empty command - check your eunice.toml configuration",
                name
            ));
        }

        let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
        if verbose {
            eprintln!("  [verbose] Spawning process: {} {:?}", command, args);
            eprintln!("  [verbose] Timeout: {}s", timeout);
        }

        // Note: stderr is set to null to prevent deadlock - if the MCP server
        // writes too much to stderr and we don't read it, the buffer fills up
        // and the server blocks on write, causing a timeout.
        let spawn_result = tokio::process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn();

        let mut process = match spawn_result {
            Ok(p) => p,
            Err(e) => {
                let error_msg = match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        format!(
                            "Command '{}' not found. Is it installed and in your PATH?",
                            command
                        )
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        format!(
                            "Permission denied executing '{}'. Check file permissions.",
                            command
                        )
                    }
                    _ => format!("{}", e),
                };
                if verbose {
                    eprintln!("  [verbose] Spawn failed: {} (kind: {:?})", e, e.kind());
                }
                return Err(anyhow!(
                    "Failed to start MCP server '{}': {} - {}",
                    name,
                    error_msg,
                    format!("command: '{}', args: {:?}", command, args)
                ));
            }
        };

        if verbose {
            eprintln!("  [verbose] Process spawned successfully");
        }

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin for MCP server '{}'", name))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout for MCP server '{}'", name))?;

        Ok(Self {
            name: name.to_string(),
            process,
            stdin,
            stdout: BufReader::new(stdout),
            timeout_secs: timeout,
        })
    }

    /// Initialize the spawned server and convert to ready McpServer
    /// Uses retry with exponential backoff for servers that need startup time
    pub async fn initialize(self, verbose: bool) -> Result<McpServer> {
        let mut server = McpServer {
            name: self.name,
            process: self.process,
            stdin: self.stdin,
            stdout: self.stdout,
            tools: Vec::new(),
            request_id: AtomicI64::new(1),
            timeout_secs: self.timeout_secs,
        };

        // Try to initialize with retries (servers may need time to start)
        let max_retries = 5;
        let mut delay_ms = 100; // Start with 100ms

        for attempt in 1..=max_retries {
            match server.initialize_protocol().await {
                Ok(()) => {
                    // Success - discover tools and return
                    server.discover_tools(verbose).await?;
                    return Ok(server);
                }
                Err(_) if attempt < max_retries => {
                    // Retry with backoff
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(1000); // Cap at 1 second
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }
}

/// Represents a single MCP server process
pub struct McpServer {
    pub name: String,
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    pub tools: Vec<Tool>,
    request_id: AtomicI64,
    timeout_secs: u64,
}

impl McpServer {
    /// Send initialize handshake
    async fn initialize_protocol(&mut self) -> Result<()> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id(),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "eunice",
                    "version": "0.1.0"
                }
            })),
        };

        self.send_message(&serde_json::to_value(&request)?).await?;
        let _response = self.read_message().await?;

        // Send initialized notification
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
        };

        self.send_message(&serde_json::to_value(&notification)?)
            .await?;

        Ok(())
    }

    /// Discover available tools from the server
    async fn discover_tools(&mut self, verbose: bool) -> Result<()> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id(),
            method: "tools/list".to_string(),
            params: None,
        };

        self.send_message(&serde_json::to_value(&request)?).await?;
        let response = self.read_message().await?;

        let json_response: JsonRpcResponse = serde_json::from_value(response)?;

        if let Some(result) = json_response.result {
            let tools_result: McpToolsResult = serde_json::from_value(result)?;

            for mcp_tool in tools_result.tools {
                let prefixed_name = format!("{}_{}", self.name, mcp_tool.name);
                let (sanitized_name, was_modified) = sanitize_tool_name(&prefixed_name);

                let mut parameters = mcp_tool.input_schema.unwrap_or(serde_json::json!({
                    "type": "object",
                    "properties": {}
                }));
                // Remove x-* extension fields that Gemini doesn't support
                sanitize_schema(&mut parameters);

                if verbose {
                    if was_modified {
                        eprintln!(
                            "  [verbose] MCP tool registered: '{}' -> '{}' (sanitized)",
                            prefixed_name, sanitized_name
                        );
                    } else {
                        eprintln!(
                            "  [verbose] MCP tool registered: '{}'",
                            sanitized_name
                        );
                    }
                    eprintln!(
                        "  [verbose]   schema: {}",
                        serde_json::to_string(&parameters).unwrap_or_default()
                    );
                }

                let tool = Tool {
                    tool_type: "function".to_string(),
                    function: FunctionSpec {
                        name: sanitized_name,
                        description: mcp_tool.description.unwrap_or_default(),
                        parameters,
                    },
                };
                self.tools.push(tool);
            }
        }

        Ok(())
    }

    /// Call a tool on this server
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<String> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id(),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": tool_name,
                "arguments": arguments
            })),
        };

        self.send_message(&serde_json::to_value(&request)?).await?;
        let response = self.read_message().await?;

        let json_response: JsonRpcResponse = serde_json::from_value(response)?;

        if let Some(error) = json_response.error {
            return Err(anyhow!("Tool error: {} ({})", error.message, error.code));
        }

        if let Some(result) = json_response.result {
            // Try to parse as MCP tool result with content blocks
            if let Ok(tool_result) = serde_json::from_value::<McpToolResult>(result.clone()) {
                let text_parts: Vec<String> = tool_result
                    .content
                    .into_iter()
                    .filter_map(|block| block.text)
                    .collect();
                return Ok(text_parts.join("\n"));
            }

            // Fallback: return the raw result as string
            Ok(serde_json::to_string_pretty(&result)?)
        } else {
            Ok("".to_string())
        }
    }

    /// Send a JSON-RPC message to the server
    async fn send_message(&mut self, message: &serde_json::Value) -> Result<()> {
        let mut msg = serde_json::to_string(message)?;
        msg.push('\n');
        self.stdin
            .write_all(msg.as_bytes())
            .await
            .context("Failed to write to MCP server")?;
        self.stdin
            .flush()
            .await
            .context("Failed to flush MCP server stdin")?;
        Ok(())
    }

    /// Read a JSON-RPC message from the server
    async fn read_message(&mut self) -> Result<serde_json::Value> {
        let read_timeout = Duration::from_secs(self.timeout_secs);

        loop {
            let mut line = String::new();
            let result = timeout(read_timeout, self.stdout.read_line(&mut line)).await;

            match result {
                Ok(Ok(0)) => {
                    return Err(anyhow!("MCP server '{}' closed connection", self.name));
                }
                Ok(Ok(_)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str(trimmed) {
                        Ok(value) => return Ok(value),
                        Err(_) => {
                            // Skip non-JSON lines (could be debug output)
                            continue;
                        }
                    }
                }
                Ok(Err(e)) => {
                    return Err(anyhow!("Failed to read from MCP server '{}': {}", self.name, e));
                }
                Err(_) => {
                    return Err(anyhow!(
                        "Timeout reading from MCP server '{}' after {} seconds",
                        self.name,
                        read_timeout.as_secs()
                    ));
                }
            }
        }
    }

    /// Get the next request ID
    fn next_id(&self) -> i64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Stop the MCP server
    pub async fn stop(&mut self) -> Result<()> {
        // Try graceful shutdown - kill the process
        let shutdown_timeout = Duration::from_secs(2);

        // First try to kill gracefully
        self.process.kill().await.ok();

        match timeout(shutdown_timeout, self.process.wait()).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(anyhow!("Error waiting for MCP server '{}': {}", self.name, e)),
            Err(_) => {
                // Force kill if timeout (already killed above, just wait)
                Ok(())
            }
        }
    }
}
