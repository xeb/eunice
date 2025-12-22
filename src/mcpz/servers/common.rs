use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// JSON-RPC request structure
#[derive(Deserialize, Debug)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC response structure
#[derive(Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }

    pub fn parse_error(message: String) -> Self {
        Self::error(None, -32700, message)
    }

    pub fn method_not_found(id: Option<serde_json::Value>, method: &str) -> Self {
        Self::error(id, -32601, format!("Method not found: {}", method))
    }

    pub fn internal_error(id: Option<serde_json::Value>, message: String) -> Self {
        Self::error(id, -32603, message)
    }
}

/// JSON-RPC error structure
#[derive(Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// MCP tool definition
#[derive(Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Create a text content response for MCP tools
pub fn text_content(text: &str) -> serde_json::Value {
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    })
}

/// Create an error content response for MCP tools
pub fn error_content(message: &str) -> serde_json::Value {
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": format!("Error: {}", message)
        }],
        "isError": true
    })
}

/// Create an image content response for MCP tools (base64 encoded)
#[allow(dead_code)]
pub fn image_content(base64_data: &str, mime_type: &str) -> serde_json::Value {
    serde_json::json!({
        "content": [{
            "type": "image",
            "data": base64_data,
            "mimeType": mime_type
        }]
    })
}

/// MCP server runner trait - implement this for each server type
pub trait McpServer {
    /// Get the server name
    fn name(&self) -> &str;

    /// Get the server version
    fn version(&self) -> &str;

    /// Get the list of tools this server provides
    fn tools(&self) -> Vec<McpTool>;

    /// Handle a tool call
    fn call_tool(&self, name: &str, arguments: &serde_json::Value) -> Result<serde_json::Value>;

    /// Whether verbose logging is enabled
    fn verbose(&self) -> bool;

    /// Log a message if verbose is enabled
    fn log(&self, message: &str) {
        if self.verbose() {
            eprintln!("[mcpz] {}", message);
        }
    }

    /// Handle the initialize request
    fn handle_initialize(&self) -> serde_json::Value {
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": self.name(),
                "version": self.version()
            }
        })
    }

    /// Handle the tools/list request
    fn handle_tools_list(&self) -> serde_json::Value {
        serde_json::json!({
            "tools": self.tools()
        })
    }

    /// Handle the tools/call request
    fn handle_tools_call(&self, params: &serde_json::Value) -> Result<serde_json::Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        self.call_tool(name, &arguments)
    }

    /// Handle a JSON-RPC request
    fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        match req.method.as_str() {
            "initialize" => Some(JsonRpcResponse::success(req.id, self.handle_initialize())),
            "initialized" | "notifications/initialized" => None,
            "tools/list" => Some(JsonRpcResponse::success(req.id, self.handle_tools_list())),
            "tools/call" => match self.handle_tools_call(&req.params) {
                Ok(result) => Some(JsonRpcResponse::success(req.id, result)),
                Err(e) => Some(JsonRpcResponse::internal_error(req.id, e.to_string())),
            },
            _ => Some(JsonRpcResponse::method_not_found(req.id, &req.method)),
        }
    }

    /// Run the server main loop
    fn run(&self) -> Result<()> {
        self.log(&format!("{} server started", self.name()));

        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    self.log(&format!("Error reading stdin: {}", e));
                    break;
                }
            };

            if line.is_empty() {
                continue;
            }

            self.log(&format!("Received: {}", line));

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    self.log(&format!("Parse error: {}", e));
                    let error_response = JsonRpcResponse::parse_error(format!("Parse error: {}", e));
                    let response_json = serde_json::to_string(&error_response)?;
                    writeln!(stdout, "{}", response_json)?;
                    stdout.flush()?;
                    continue;
                }
            };

            if let Some(response) = self.handle_request(request) {
                let response_json = serde_json::to_string(&response)?;
                self.log(&format!("Sending: {}", response_json));
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
            }
        }

        self.log(&format!("{} server stopped", self.name()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_response_success() {
        let resp = JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!({"test": true}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let resp = JsonRpcResponse::error(Some(serde_json::json!(1)), -32600, "Invalid Request".to_string());
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
    }

    #[test]
    fn test_json_rpc_request_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(serde_json::json!(1)));
        assert_eq!(req.method, "initialize");
    }

    #[test]
    fn test_text_content() {
        let content = text_content("Hello, World!");
        assert_eq!(content["content"][0]["type"], "text");
        assert_eq!(content["content"][0]["text"], "Hello, World!");
    }

    #[test]
    fn test_error_content() {
        let content = error_content("Something went wrong");
        assert_eq!(content["content"][0]["type"], "text");
        assert!(content["content"][0]["text"].as_str().unwrap().contains("Error:"));
        assert_eq!(content["isError"], true);
    }

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpTool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"test_tool\""));
        assert!(json.contains("\"inputSchema\""));
    }
}
