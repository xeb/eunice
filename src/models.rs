use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provider types supported by eunice
#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    OpenAI,
    Gemini,
    Anthropic,
    Ollama,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::OpenAI => write!(f, "OpenAI"),
            Provider::Gemini => write!(f, "Gemini"),
            Provider::Anthropic => write!(f, "Anthropic"),
            Provider::Ollama => write!(f, "Ollama"),
        }
    }
}

/// Information about a detected provider
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub provider: Provider,
    pub base_url: String,
    pub api_key: String,
    pub resolved_model: String,
}

/// Message types for conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User { content: String },
    #[serde(rename = "assistant")]
    Assistant {
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    #[serde(rename = "tool")]
    Tool {
        tool_call_id: String,
        content: String,
    },
}

/// A tool call made by the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string
}

/// Tool specification for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionSpec,
}

/// Function specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Chat completion request
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

/// Chat completion response
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

/// A choice in the response
#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: AssistantMessage,
}

/// Assistant message from the API
#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// MCP configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
}

/// JSON-RPC request
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC notification (no id)
#[derive(Debug, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
}

/// JSON-RPC response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<i64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

/// MCP tool list response
#[derive(Debug, Deserialize)]
pub struct McpToolsResult {
    pub tools: Vec<McpToolSpec>,
}

/// MCP tool specification
#[derive(Debug, Clone, Deserialize)]
pub struct McpToolSpec {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<serde_json::Value>,
}

/// MCP tool call result
#[derive(Debug, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContentBlock>,
}

/// MCP content block
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct McpContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

/// JSON-RPC event for --events mode
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct JsonRpcEvent {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[allow(dead_code)]
impl JsonRpcEvent {
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }
}

/// Ollama API tags response
#[derive(Debug, Deserialize)]
pub struct OllamaTagsResponse {
    pub models: Vec<OllamaModel>,
}

/// Ollama model info
#[derive(Debug, Deserialize)]
pub struct OllamaModel {
    pub name: String,
}
