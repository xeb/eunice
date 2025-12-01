use crate::mcp::{sanitize_schema, sanitize_tool_name, warn_if_tool_name_too_long};
use crate::models::{
    FunctionSpec, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpToolResult,
    McpToolsResult, Tool,
};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

/// Represents an MCP server connected via Streamable HTTP transport
pub struct HttpMcpServer {
    pub name: String,
    url: String,
    client: Client,
    session_id: Option<String>,
    pub tools: Vec<Tool>,
    request_id: AtomicI64,
}

/// Default timeout in seconds for MCP requests (10 minutes)
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

impl HttpMcpServer {
    /// Connect to an HTTP MCP server and initialize it
    pub async fn connect(name: &str, url: &str, timeout_secs: Option<u64>, verbose: bool) -> Result<Self> {
        let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
        if verbose {
            eprintln!("  [verbose] HTTP timeout: {}s", timeout);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .context("Failed to create HTTP client")?;

        let mut server = Self {
            name: name.to_string(),
            url: url.to_string(),
            client,
            session_id: None,
            tools: Vec::new(),
            request_id: AtomicI64::new(1),
        };

        // Initialize the MCP protocol
        server.initialize_protocol().await.map_err(|e| {
            // Provide detailed error with root cause
            let cause = e.root_cause();
            anyhow::anyhow!(
                "Failed to connect to HTTP MCP server '{}' at {}: {}",
                name,
                url,
                cause
            )
        })?;

        // Discover available tools
        server
            .discover_tools(verbose)
            .await
            .with_context(|| format!("Failed to discover tools from HTTP MCP server '{}'", name))?;

        Ok(server)
    }

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
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        };

        let response = self.send_request(&request).await?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "MCP initialize failed: {} (code: {})",
                error.message,
                error.code
            ));
        }

        // Send initialized notification
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
        };

        // Send notification (no response expected, but HTTP requires we read it)
        let _ = self.send_notification(&notification).await;

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

        let response = self.send_request(&request).await?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "Failed to list tools: {} (code: {})",
                error.message,
                error.code
            ));
        }

        if let Some(result) = response.result {
            let tools_result: McpToolsResult = serde_json::from_value(result)
                .context("Failed to parse tools/list response")?;

            for mcp_tool in tools_result.tools {
                // Prefix tool name with server name for routing
                let prefixed_name = format!("{}_{}", self.name, mcp_tool.name);
                let (sanitized_name, was_modified) = sanitize_tool_name(&prefixed_name);

                // Warn if tool name exceeds Gemini's limit
                warn_if_tool_name_too_long(&sanitized_name, &self.name);

                let mut parameters = mcp_tool.input_schema.unwrap_or(serde_json::json!({
                    "type": "object",
                    "properties": {}
                }));
                // Remove x-* extension fields that Gemini doesn't support
                sanitize_schema(&mut parameters);

                if verbose {
                    if was_modified {
                        eprintln!(
                            "  [verbose] HTTP MCP tool registered: '{}' -> '{}' (sanitized)",
                            prefixed_name, sanitized_name
                        );
                    } else {
                        eprintln!(
                            "  [verbose] HTTP MCP tool registered: '{}'",
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

        let response = self
            .send_request(&request)
            .await
            .with_context(|| format!("HTTP request to '{}' failed for tool '{}'", self.url, tool_name))?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "Tool '{}' error: {} (code: {})",
                tool_name,
                error.message,
                error.code
            ));
        }

        if let Some(result) = response.result {
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

    /// Send a JSON-RPC request and get the response
    async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        let mut req_builder = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        // Add session ID if we have one
        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("mcp-session-id", session_id);
        }

        let response = req_builder
            .json(request)
            .send()
            .await
            .with_context(|| format!("Failed to connect to MCP server at {}", self.url))?;

        // Check for HTTP errors
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "HTTP {} from MCP server '{}': {}",
                status,
                self.name,
                if error_body.is_empty() {
                    status.canonical_reason().unwrap_or("Unknown error")
                } else {
                    &error_body
                }
            ));
        }

        // Extract session ID from response headers if present
        if let Some(session_id) = response.headers().get("mcp-session-id") {
            if let Ok(id) = session_id.to_str() {
                self.session_id = Some(id.to_string());
            }
        }

        let json_response: JsonRpcResponse = response
            .json()
            .await
            .context("Failed to parse JSON-RPC response")?;

        Ok(json_response)
    }

    /// Send a JSON-RPC notification (no response expected, but HTTP returns one anyway)
    async fn send_notification(&mut self, notification: &JsonRpcNotification) -> Result<()> {
        let mut req_builder = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("mcp-session-id", session_id);
        }

        let response = req_builder.json(notification).send().await?;

        // Just consume the response, don't care about content for notifications
        let _ = response.text().await;
        Ok(())
    }

    /// Get the next request ID
    fn next_id(&self) -> i64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }
}
