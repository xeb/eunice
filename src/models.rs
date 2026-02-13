use serde::{Deserialize, Serialize};

/// Provider types supported by eunice
#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    OpenAI,
    Gemini,
    Anthropic,
    Ollama,
    AzureOpenAI,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::OpenAI => write!(f, "OpenAI"),
            Provider::Gemini => write!(f, "Gemini"),
            Provider::Anthropic => write!(f, "Anthropic"),
            Provider::Ollama => write!(f, "Ollama"),
            Provider::AzureOpenAI => write!(f, "Azure OpenAI"),
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
            Provider::AzureOpenAI => "☁️",
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
    /// Azure OpenAI API version (e.g., "2024-02-01")
    pub azure_api_version: Option<String>,
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
    /// Token usage statistics (optional, not all providers return this)
    #[serde(default)]
    pub usage: Option<UsageStats>,
}

/// Token usage statistics from API response
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UsageStats {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    /// Cached tokens (Anthropic, some OpenAI models)
    #[serde(default, alias = "cache_read_input_tokens")]
    pub cached_tokens: u64,
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

/// Configuration for the webapp server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebappConfig {
    /// Host to bind to (default: "0.0.0.0")
    #[serde(default = "default_webapp_host")]
    pub host: String,
    /// Port to bind to (default: 8811)
    #[serde(default = "default_webapp_port")]
    pub port: u16,
}

fn default_webapp_host() -> String {
    "0.0.0.0".to_string()
}

fn default_webapp_port() -> u16 {
    8811
}

impl Default for WebappConfig {
    fn default() -> Self {
        Self {
            host: default_webapp_host(),
            port: default_webapp_port(),
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

/// Gemini tool container - supports function declarations and code execution
#[derive(Debug, Serialize)]
pub struct GeminiTool {
    #[serde(rename = "functionDeclarations", skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<GeminiFunctionDeclaration>>,
    /// Agentic Vision code execution tool (empty object to enable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<serde_json::Value>,
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
    /// Token usage metadata
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

/// Gemini usage metadata
#[derive(Debug, Deserialize)]
pub struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    pub prompt_token_count: u64,
    #[serde(rename = "candidatesTokenCount", default)]
    pub candidates_token_count: u64,
    #[serde(rename = "totalTokenCount", default)]
    pub total_token_count: u64,
    #[serde(rename = "cachedContentTokenCount", default)]
    pub cached_content_token_count: u64,
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
    /// Agentic Vision: code generated and executed by the model
    #[serde(rename = "executableCode")]
    pub executable_code: Option<GeminiExecutableCode>,
    /// Agentic Vision: result of code execution
    #[serde(rename = "codeExecutionResult")]
    pub code_execution_result: Option<GeminiCodeExecutionResult>,
}

/// Agentic Vision: executable code generated by Gemini
#[derive(Debug, Deserialize)]
pub struct GeminiExecutableCode {
    pub language: String,
    pub code: String,
}

/// Agentic Vision: result of code execution
#[derive(Debug, Deserialize)]
pub struct GeminiCodeExecutionResult {
    pub outcome: String,
    #[serde(default)]
    pub output: Option<String>,
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
