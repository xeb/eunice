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

impl Provider {
    pub fn get_icon(&self) -> &'static str {
        match self {
            Provider::OpenAI => "🤖",
            Provider::Gemini => "💎",
            Provider::Anthropic => "🧠",
            Provider::Ollama => "🦙",
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
    pub use_native_gemini_api: bool,
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
    pub messages: serde_json::Value,
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,
    /// Optional list of allowed tool names (full sanitized names like "server_toolname")
    /// If empty or not specified, all tools are allowed
    #[serde(rename = "allowedTools", default)]
    pub allowed_tools: Vec<String>,
}

/// Configuration for a single agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// System prompt (inline string or file path)
    pub prompt: String,
    /// Tool name patterns this agent can access (supports * wildcard)
    /// Examples: "eng_file_read" (exact), "eng_*" (all eng tools), "*_read" (all read tools)
    #[serde(default)]
    pub tools: Vec<String>,
    /// Agent names this agent can invoke
    #[serde(default)]
    pub can_invoke: Vec<String>,
}

/// Configuration for a single MCP server
/// Supports two transport types:
/// - Stdio: command + args (spawn a subprocess)
/// - HTTP: url (connect to remote Streamable HTTP server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to spawn (for stdio transport)
    #[serde(default)]
    pub command: String,
    /// Arguments for the command (for stdio transport)
    #[serde(default)]
    pub args: Vec<String>,
    /// URL for HTTP transport (e.g., "http://localhost:3323/mcp")
    #[serde(default)]
    pub url: Option<String>,
    /// Timeout in seconds for requests (default: 60)
    #[serde(default)]
    pub timeout: Option<u64>,
}

impl McpServerConfig {
    /// Check if this is an HTTP-based server
    pub fn is_http(&self) -> bool {
        self.url.is_some()
    }
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

// Native Gemini API structures

/// Gemini API request format
#[derive(Debug, Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,
}

/// Gemini tool container
#[derive(Debug, Serialize)]
pub struct GeminiTool {
    #[serde(rename = "functionDeclarations")]
    pub function_declarations: Vec<GeminiFunctionDeclaration>,
}

/// Gemini function declaration (equivalent to OpenAI's function spec)
#[derive(Debug, Serialize)]
pub struct GeminiFunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Gemini content structure
#[derive(Debug, Serialize)]
pub struct GeminiContent {
    pub parts: Vec<GeminiPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Gemini part structure (for requests)
#[derive(Debug, Serialize)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<GeminiInlineData>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    pub function_call: Option<GeminiFunctionCallRequest>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    pub function_response: Option<GeminiFunctionResponse>,
    /// Thought signature for Gemini 3 models - must be passed back with function calls
    #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

/// Gemini inline data for images
#[derive(Debug, Serialize)]
pub struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String, // base64
}

/// Gemini function call (for requests, when continuing a conversation)
#[derive(Debug, Serialize)]
pub struct GeminiFunctionCallRequest {
    pub name: String,
    pub args: serde_json::Value,
}

/// Gemini function response (tool result)
#[derive(Debug, Serialize)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

/// Gemini API response format
#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    /// Feedback about the prompt (may contain block reason)
    #[serde(rename = "promptFeedback")]
    pub prompt_feedback: Option<GeminiPromptFeedback>,
}

/// Gemini candidate structure
#[derive(Debug, Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiContentResponse,
    /// Why the model stopped generating
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    /// Additional detail about the finish reason
    #[serde(rename = "finishMessage")]
    pub finish_message: Option<String>,
}

/// Gemini prompt feedback (may indicate blocked prompts)
#[derive(Debug, Deserialize)]
pub struct GeminiPromptFeedback {
    /// Block reason if the prompt was blocked
    #[serde(rename = "blockReason")]
    pub block_reason: Option<String>,
    /// Safety ratings
    #[serde(rename = "safetyRatings")]
    pub safety_ratings: Option<Vec<GeminiSafetyRating>>,
}

/// Gemini safety rating
#[derive(Debug, Deserialize)]
pub struct GeminiSafetyRating {
    pub category: String,
    pub probability: String,
}

/// Gemini content response structure
#[derive(Debug, Deserialize)]
pub struct GeminiContentResponse {
    #[serde(default)]
    pub parts: Vec<GeminiPartResponse>,
}

/// Gemini part response structure
#[derive(Debug, Deserialize)]
pub struct GeminiPartResponse {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "functionCall")]
    pub function_call: Option<GeminiFunctionCall>,
    /// Thought signature for Gemini 3 models - must be passed back with function responses
    #[serde(rename = "thoughtSignature")]
    pub thought_signature: Option<String>,
}

/// Gemini function call in response
#[derive(Debug, Deserialize)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_request_serialization() {
        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: Some("Hello".to_string()),
                    function_call: None,
                    function_response: None,
                    thought_signature: None,
                    inline_data: None,
                }],
                role: Some("user".to_string()),
            }],
            tools: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("contents"));
        assert!(json.contains("parts"));
        assert!(json.contains("Hello"));
        assert!(json.contains("user"));
    }

    #[test]
    fn test_gemini_response_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "parts": [{"text": "Response text"}]
                }
            }]
        }"#;

        let response: GeminiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.candidates[0].content.parts.len(), 1);
        assert_eq!(response.candidates[0].content.parts[0].text, Some("Response text".to_string()));
    }

    #[test]
    fn test_gemini_request_without_role() {
        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: Some("Test".to_string()),
                    function_call: None,
                    function_response: None,
                    thought_signature: None,
                    inline_data: None,
                }],
                role: None,
            }],
            tools: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        // Role should not be present when None
        assert!(!json.contains("role"));
    }

    #[test]
    fn test_message_user_serialization() {
        let message = Message::User {
            content: "Test content".to_string(),
        };

        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Test content");
    }

    #[test]
    fn test_message_assistant_serialization() {
        let message = Message::Assistant {
            content: Some("Response".to_string()),
            tool_calls: None,
        };

        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(json["role"], "assistant");
        assert_eq!(json["content"], "Response");
    }

    #[test]
    fn test_message_tool_serialization() {
        let message = Message::Tool {
            tool_call_id: "call_123".to_string(),
            content: "Tool result".to_string(),
        };

        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(json["role"], "tool");
        assert_eq!(json["tool_call_id"], "call_123");
        assert_eq!(json["content"], "Tool result");
    }
}
