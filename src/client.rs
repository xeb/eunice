use crate::models::{
    ChatCompletionRequest, ChatCompletionResponse, GeminiContent, GeminiPart, GeminiRequest,
    GeminiResponse, Message, Provider, ProviderInfo, Tool,
};
use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

/// OpenAI-compatible HTTP client for all providers
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    provider: Provider,
    use_native_gemini_api: bool,
}

impl Client {
    /// Create a new client for the given provider
    pub fn new(provider_info: &ProviderInfo) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Set authorization header based on provider
        // Skip default auth headers for native Gemini API (uses per-request x-goog-api-key)
        if !provider_info.use_native_gemini_api {
            match provider_info.provider {
                Provider::Anthropic => {
                    headers.insert(
                        "x-api-key",
                        HeaderValue::from_str(&provider_info.api_key)
                            .context("Invalid Anthropic API key")?,
                    );
                    headers.insert(
                        "anthropic-version",
                        HeaderValue::from_static("2023-06-01"),
                    );
                }
                Provider::Ollama => {
                    // Ollama doesn't need auth
                }
                _ => {
                    headers.insert(
                        AUTHORIZATION,
                        HeaderValue::from_str(&format!("Bearer {}", provider_info.api_key))
                            .context("Invalid API key format")?,
                    );
                }
            }
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http,
            base_url: provider_info.base_url.clone(),
            api_key: provider_info.api_key.clone(),
            provider: provider_info.provider.clone(),
            use_native_gemini_api: provider_info.use_native_gemini_api,
        })
    }

    /// Send a chat completion request
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
        dmn_mode: bool,
    ) -> Result<ChatCompletionResponse> {
        // Check if using native Gemini API
        if self.use_native_gemini_api {
            return self.chat_completion_gemini_native(model, messages, tools, dmn_mode).await;
        }

        // Standard OpenAI-compatible API
        let url = format!("{}chat/completions", self.base_url);

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            tool_choice: tools.map(|_| "auto".to_string()),
        };

        // Try request with potential retry on 429
        let mut attempt = 0;
        loop {
            attempt += 1;

            let response = self
                .http
                .post(&url)
                .json(&request)
                .send()
                .await
                .context("Failed to send request")?;

            let status = response.status();

            // Handle 429 rate limit in DMN mode
            if status.as_u16() == 429 && dmn_mode && attempt == 1 {
                let error_text = response.text().await.unwrap_or_default();
                eprintln!("⏳ Rate limit hit (429). DMN mode: retrying in 6 seconds...");
                eprintln!("   Error: {}", error_text.lines().next().unwrap_or("Unknown error"));
                tokio::time::sleep(Duration::from_secs(6)).await;
                continue; // Retry
            }

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(anyhow!(
                    "API request failed with status {}: {}",
                    status,
                    error_text
                ));
            }

            let response_body = response
                .json::<ChatCompletionResponse>()
                .await
                .context("Failed to parse response")?;

            return Ok(response_body);
        }
    }

    /// Send a chat completion request using native Gemini API
    async fn chat_completion_gemini_native(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
        dmn_mode: bool,
    ) -> Result<ChatCompletionResponse> {
        use crate::models::{GeminiFunctionDeclaration, GeminiTool};

        // Convert messages to Gemini format
        let contents = self.convert_messages_to_gemini(messages)?;

        // Convert OpenAI-style tools to Gemini functionDeclarations
        // Gemini doesn't support all JSON Schema properties, so we need to strip unsupported ones
        let gemini_tools = tools.map(|t| {
            let declarations: Vec<GeminiFunctionDeclaration> = t
                .iter()
                .map(|tool| GeminiFunctionDeclaration {
                    name: tool.function.name.clone(),
                    description: tool.function.description.clone(),
                    parameters: Self::clean_schema_for_gemini(&tool.function.parameters),
                })
                .collect();
            vec![GeminiTool {
                function_declarations: declarations,
            }]
        });

        let gemini_request = GeminiRequest {
            contents,
            tools: gemini_tools,
        };

        // Build URL: base_url already contains /v1beta/models/
        let url = format!("{}{}:generateContent", self.base_url, model);

        // Try request with potential retry on 429
        let mut attempt = 0;
        loop {
            attempt += 1;

            // Build request with x-goog-api-key header
            let response = self
                .http
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .header(CONTENT_TYPE, "application/json")
                .json(&gemini_request)
                .send()
                .await
                .context("Failed to send Gemini request")?;

            let status = response.status();

            // Handle 429 rate limit in DMN mode
            if status.as_u16() == 429 && dmn_mode && attempt == 1 {
                let error_text = response.text().await.unwrap_or_default();
                eprintln!("⏳ Rate limit hit (429). DMN mode: retrying in 6 seconds...");
                eprintln!("   Error: {}", error_text.lines().next().unwrap_or("Unknown error"));
                tokio::time::sleep(Duration::from_secs(6)).await;
                continue; // Retry
            }

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(anyhow!(
                    "Gemini API request failed with status {}: {}",
                    status,
                    error_text
                ));
            }

            // Get raw response text first so we can inspect it
            let response_text = response
                .text()
                .await
                .context("Failed to read Gemini response body")?;

            // Parse the response
            let gemini_response: GeminiResponse = serde_json::from_str(&response_text)
                .context("Failed to parse Gemini response JSON")?;

            // Convert Gemini response to OpenAI format, passing raw text for debugging
            return self.convert_gemini_to_openai_response(gemini_response, &response_text);
        }
    }

    /// Convert OpenAI-style messages to Gemini contents format
    fn convert_messages_to_gemini(&self, messages: &[Message]) -> Result<Vec<GeminiContent>> {
        use crate::models::{GeminiFunctionCallRequest, GeminiFunctionResponse};

        let mut contents = Vec::new();

        for message in messages {
            match message {
                Message::User { content } => {
                    contents.push(GeminiContent {
                        parts: vec![GeminiPart {
                            text: Some(content.clone()),
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        }],
                        role: Some("user".to_string()),
                    });
                }
                Message::Assistant { content, tool_calls } => {
                    let mut parts = Vec::new();

                    // Add text content if present
                    if let Some(text) = content {
                        if !text.is_empty() {
                            parts.push(GeminiPart {
                                text: Some(text.clone()),
                                function_call: None,
                                function_response: None,
                                thought_signature: None,
                            });
                        }
                    }

                    // Add function calls if present
                    // Extract thought_signature from encoded ID (format: "name::signature")
                    if let Some(calls) = tool_calls {
                        for call in calls {
                            let args: serde_json::Value =
                                serde_json::from_str(&call.function.arguments).unwrap_or_default();

                            // Extract thought_signature from ID if present
                            let thought_signature = if call.id.contains("::") {
                                call.id.split("::").nth(1).map(|s| s.to_string())
                            } else {
                                None
                            };

                            parts.push(GeminiPart {
                                text: None,
                                function_call: Some(GeminiFunctionCallRequest {
                                    name: call.function.name.clone(),
                                    args,
                                }),
                                function_response: None,
                                thought_signature,
                            });
                        }
                    }

                    if !parts.is_empty() {
                        contents.push(GeminiContent {
                            parts,
                            role: Some("model".to_string()),
                        });
                    }
                }
                Message::Tool { tool_call_id, content } => {
                    // Convert tool result to Gemini functionResponse format
                    // Try to parse the content as JSON, otherwise wrap it
                    let response_value: serde_json::Value =
                        serde_json::from_str(content).unwrap_or_else(|_| {
                            serde_json::json!({ "result": content })
                        });

                    // Extract function name from tool_call_id
                    // The ID may be encoded as "name::signature" or just "name"
                    let function_name = tool_call_id.split("::").next().unwrap_or(tool_call_id).to_string();

                    contents.push(GeminiContent {
                        parts: vec![GeminiPart {
                            text: None,
                            function_call: None,
                            function_response: Some(GeminiFunctionResponse {
                                name: function_name,
                                response: response_value,
                            }),
                            thought_signature: None,
                        }],
                        role: Some("user".to_string()),
                    });
                }
            }
        }

        Ok(contents)
    }

    /// Convert Gemini response to OpenAI-compatible format
    fn convert_gemini_to_openai_response(
        &self,
        gemini_response: GeminiResponse,
        raw_response: &str,
    ) -> Result<ChatCompletionResponse> {
        // Check for prompt-level blocking first
        if let Some(ref feedback) = gemini_response.prompt_feedback {
            if let Some(ref block_reason) = feedback.block_reason {
                eprintln!("⛔ Gemini blocked the prompt: {}", block_reason);
                return Err(anyhow!("Prompt blocked by Gemini: {}", block_reason));
            }
        }

        if gemini_response.candidates.is_empty() {
            // If no candidates but we have prompt feedback, show safety ratings
            if let Some(ref feedback) = gemini_response.prompt_feedback {
                if let Some(ref ratings) = feedback.safety_ratings {
                    let high_ratings: Vec<_> = ratings
                        .iter()
                        .filter(|r| r.probability == "HIGH" || r.probability == "MEDIUM")
                        .map(|r| format!("{}: {}", r.category, r.probability))
                        .collect();
                    if !high_ratings.is_empty() {
                        eprintln!("⚠️  Gemini safety concerns: {}", high_ratings.join(", "));
                    }
                }
            }
            return Err(anyhow!("Gemini response has no candidates"));
        }

        let candidate = &gemini_response.candidates[0];

        // Report non-normal finish reasons
        if let Some(ref reason) = candidate.finish_reason {
            match reason.as_str() {
                "STOP" => {} // Normal completion
                "MAX_TOKENS" => {
                    eprintln!("⚠️  Gemini stopped: reached maximum token limit");
                }
                "SAFETY" => {
                    eprintln!("⛔ Gemini stopped: safety filters triggered");
                }
                "RECITATION" => {
                    eprintln!("⛔ Gemini stopped: recitation/copyright concern");
                }
                "OTHER" => {
                    eprintln!("⚠️  Gemini stopped: unspecified reason (OTHER)");
                }
                "MALFORMED_FUNCTION_CALL" => {
                    eprintln!("⚠️  Gemini stopped: model tried to call a function but tools are not enabled for this API path");
                    if let Some(ref msg) = candidate.finish_message {
                        eprintln!("   {}", msg);
                    }
                }
                _ => {
                    eprintln!("⚠️  Gemini stopped with reason: {}", reason);
                    // Show finish message if available
                    if let Some(ref msg) = candidate.finish_message {
                        eprintln!("   {}", msg);
                    } else {
                        // For unexpected reasons without a message, dump the raw response
                        eprintln!("📋 Raw Gemini response:\n{}", raw_response);
                    }
                }
            }
        }

        // Extract text content
        let text = candidate
            .content
            .parts
            .iter()
            .filter_map(|p| p.text.as_ref())
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Extract function calls and convert to OpenAI tool_calls format
        // Encode both function name and thought_signature in the ID for Gemini 3 compatibility
        // Format: "name::signature" or just "name" if no signature
        let tool_calls: Vec<crate::models::ToolCall> = candidate
            .content
            .parts
            .iter()
            .filter_map(|p| {
                p.function_call.as_ref().map(|fc| {
                    let id = match &p.thought_signature {
                        Some(sig) => format!("{}::{}", fc.name, sig),
                        None => fc.name.clone(),
                    };
                    crate::models::ToolCall {
                        id,
                        call_type: "function".to_string(),
                        function: crate::models::FunctionCall {
                            name: fc.name.clone(),
                            arguments: fc.args.to_string(),
                        },
                    }
                })
            })
            .collect();

        let tool_calls = if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        };

        // Build OpenAI-compatible response
        Ok(ChatCompletionResponse {
            choices: vec![crate::models::Choice {
                message: crate::models::AssistantMessage {
                    content: if text.is_empty() { None } else { Some(text) },
                    tool_calls,
                },
            }],
        })
    }

    /// Get the provider type
    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    /// Clean a JSON Schema to remove properties that Gemini doesn't support
    fn clean_schema_for_gemini(schema: &serde_json::Value) -> serde_json::Value {
        Self::clean_schema_recursive(schema, 0)
    }

    fn clean_schema_recursive(schema: &serde_json::Value, depth: usize) -> serde_json::Value {
        // Properties that Gemini doesn't support in function declarations
        const UNSUPPORTED: &[&str] = &[
            "additionalProperties",
            "$schema",
            "exclusiveMaximum",
            "exclusiveMinimum",
            "$id",
            "$ref",
            "definitions",
            "$defs",
            "default",
            "examples",
            "title",
            // JSON Schema combinators not supported by Gemini native API
            "anyOf",
            "oneOf",
            "allOf",
            "not",
        ];

        // If schema is too deeply nested, simplify it
        // Gemini has trouble with schemas nested more than ~4 levels deep
        if depth > 4 {
            return serde_json::json!({
                "type": "object",
                "description": "Complex nested object (simplified for API compatibility)"
            });
        }

        match schema {
            serde_json::Value::Object(obj) => {
                let mut cleaned = serde_json::Map::new();

                // First pass: collect all keys except unsupported ones
                for (key, value) in obj {
                    if !UNSUPPORTED.contains(&key.as_str()) {
                        cleaned.insert(key.clone(), Self::clean_schema_recursive(value, depth + 1));
                    }
                }

                // Second pass: clean up "required" array to only reference existing properties
                if let Some(serde_json::Value::Object(props)) = cleaned.get("properties") {
                    if let Some(serde_json::Value::Array(required)) = cleaned.get("required") {
                        let valid_required: Vec<serde_json::Value> = required
                            .iter()
                            .filter(|r| {
                                if let serde_json::Value::String(s) = r {
                                    props.contains_key(s)
                                } else {
                                    false
                                }
                            })
                            .cloned()
                            .collect();
                        cleaned.insert("required".to_string(), serde_json::Value::Array(valid_required));
                    }
                }

                serde_json::Value::Object(cleaned)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(
                    arr.iter().map(|v| Self::clean_schema_recursive(v, depth)).collect()
                )
            }
            other => other.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GeminiCandidate, GeminiContentResponse, GeminiPartResponse};

    fn create_test_client() -> Client {
        let provider_info = ProviderInfo {
            provider: Provider::Gemini,
            base_url: "https://test.com/".to_string(),
            api_key: "test-key".to_string(),
            resolved_model: "gemini-3-pro-preview".to_string(),
            use_native_gemini_api: true,
        };
        Client::new(&provider_info).unwrap()
    }

    #[test]
    fn test_convert_messages_to_gemini_user_message() {
        let client = create_test_client();
        let messages = vec![Message::User {
            content: "Hello, world!".to_string(),
        }];

        let result = client.convert_messages_to_gemini(&messages);
        assert!(result.is_ok());

        let contents = result.unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, Some("user".to_string()));
        assert_eq!(contents[0].parts.len(), 1);
        assert_eq!(contents[0].parts[0].text, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_convert_messages_to_gemini_assistant_message() {
        let client = create_test_client();
        let messages = vec![Message::Assistant {
            content: Some("Hi there!".to_string()),
            tool_calls: None,
        }];

        let result = client.convert_messages_to_gemini(&messages);
        assert!(result.is_ok());

        let contents = result.unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, Some("model".to_string()));
        assert_eq!(contents[0].parts.len(), 1);
        assert_eq!(contents[0].parts[0].text, Some("Hi there!".to_string()));
    }

    #[test]
    fn test_convert_messages_to_gemini_conversation() {
        let client = create_test_client();
        let messages = vec![
            Message::User {
                content: "What is 2+2?".to_string(),
            },
            Message::Assistant {
                content: Some("4".to_string()),
                tool_calls: None,
            },
            Message::User {
                content: "What is 3+3?".to_string(),
            },
        ];

        let result = client.convert_messages_to_gemini(&messages);
        assert!(result.is_ok());

        let contents = result.unwrap();
        assert_eq!(contents.len(), 3);

        // First message
        assert_eq!(contents[0].role, Some("user".to_string()));
        assert_eq!(contents[0].parts[0].text, Some("What is 2+2?".to_string()));

        // Second message
        assert_eq!(contents[1].role, Some("model".to_string()));
        assert_eq!(contents[1].parts[0].text, Some("4".to_string()));

        // Third message
        assert_eq!(contents[2].role, Some("user".to_string()));
        assert_eq!(contents[2].parts[0].text, Some("What is 3+3?".to_string()));
    }

    #[test]
    fn test_convert_messages_to_gemini_converts_tool_messages() {
        let client = create_test_client();
        let messages = vec![
            Message::User {
                content: "Test".to_string(),
            },
            Message::Tool {
                tool_call_id: "my_function".to_string(),
                content: "Tool result".to_string(),
            },
        ];

        let result = client.convert_messages_to_gemini(&messages);
        assert!(result.is_ok());

        let contents = result.unwrap();
        // Tool message is now converted to functionResponse
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role, Some("user".to_string()));
        assert_eq!(contents[1].role, Some("user".to_string())); // functionResponse uses "user" role
        assert!(contents[1].parts[0].function_response.is_some());
    }

    #[test]
    fn test_convert_gemini_to_openai_response() {
        let client = create_test_client();
        let gemini_response = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContentResponse {
                    parts: vec![GeminiPartResponse {
                        text: Some("This is a test response".to_string()),
                        function_call: None,
                        thought_signature: None,
                    }],
                },
                finish_reason: Some("STOP".to_string()),
                finish_message: None,
            }],
            prompt_feedback: None,
        };

        let result = client.convert_gemini_to_openai_response(gemini_response, "{}");
        assert!(result.is_ok());

        let openai_response = result.unwrap();
        assert_eq!(openai_response.choices.len(), 1);
        assert_eq!(
            openai_response.choices[0].message.content,
            Some("This is a test response".to_string())
        );
        assert!(openai_response.choices[0].message.tool_calls.is_none());
    }

    #[test]
    fn test_convert_gemini_to_openai_response_multiple_parts() {
        let client = create_test_client();
        let gemini_response = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContentResponse {
                    parts: vec![
                        GeminiPartResponse {
                            text: Some("First part".to_string()),
                            function_call: None,
                            thought_signature: None,
                        },
                        GeminiPartResponse {
                            text: Some("Second part".to_string()),
                            function_call: None,
                            thought_signature: None,
                        },
                    ],
                },
                finish_reason: Some("STOP".to_string()),
                finish_message: None,
            }],
            prompt_feedback: None,
        };

        let result = client.convert_gemini_to_openai_response(gemini_response, "{}");
        assert!(result.is_ok());

        let openai_response = result.unwrap();
        assert_eq!(
            openai_response.choices[0].message.content,
            Some("First part\nSecond part".to_string())
        );
    }

    #[test]
    fn test_convert_gemini_to_openai_response_no_candidates() {
        let client = create_test_client();
        let gemini_response = GeminiResponse {
            candidates: vec![],
            prompt_feedback: None,
        };

        let result = client.convert_gemini_to_openai_response(gemini_response, "{}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no candidates"));
    }
}
