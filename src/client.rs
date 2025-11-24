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
    resolved_model: String,
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
            resolved_model: provider_info.resolved_model.clone(),
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
            return self.chat_completion_gemini_native(model, messages, dmn_mode).await;
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
        dmn_mode: bool,
    ) -> Result<ChatCompletionResponse> {
        // Convert messages to Gemini format
        let contents = self.convert_messages_to_gemini(messages)?;

        let gemini_request = GeminiRequest { contents };

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

            let gemini_response = response
                .json::<GeminiResponse>()
                .await
                .context("Failed to parse Gemini response")?;

            // Convert Gemini response to OpenAI format
            return self.convert_gemini_to_openai_response(gemini_response);
        }
    }

    /// Convert OpenAI-style messages to Gemini contents format
    fn convert_messages_to_gemini(&self, messages: &[Message]) -> Result<Vec<GeminiContent>> {
        let mut contents = Vec::new();

        for message in messages {
            match message {
                Message::User { content } => {
                    contents.push(GeminiContent {
                        parts: vec![GeminiPart {
                            text: Some(content.clone()),
                        }],
                        role: Some("user".to_string()),
                    });
                }
                Message::Assistant { content, .. } => {
                    if let Some(text) = content {
                        contents.push(GeminiContent {
                            parts: vec![GeminiPart {
                                text: Some(text.clone()),
                            }],
                            role: Some("model".to_string()),
                        });
                    }
                }
                Message::Tool { .. } => {
                    // Skip tool messages for now - native Gemini API doesn't support tools yet
                }
            }
        }

        Ok(contents)
    }

    /// Convert Gemini response to OpenAI-compatible format
    fn convert_gemini_to_openai_response(
        &self,
        gemini_response: GeminiResponse,
    ) -> Result<ChatCompletionResponse> {
        if gemini_response.candidates.is_empty() {
            return Err(anyhow!("Gemini response has no candidates"));
        }

        let candidate = &gemini_response.candidates[0];
        let text = candidate
            .content
            .parts
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Build OpenAI-compatible response
        Ok(ChatCompletionResponse {
            choices: vec![crate::models::Choice {
                message: crate::models::AssistantMessage {
                    content: Some(text),
                    tool_calls: None,
                },
            }],
        })
    }

    /// Get the provider type
    pub fn provider(&self) -> &Provider {
        &self.provider
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
    fn test_convert_messages_to_gemini_skips_tool_messages() {
        let client = create_test_client();
        let messages = vec![
            Message::User {
                content: "Test".to_string(),
            },
            Message::Tool {
                tool_call_id: "123".to_string(),
                content: "Tool result".to_string(),
            },
        ];

        let result = client.convert_messages_to_gemini(&messages);
        assert!(result.is_ok());

        let contents = result.unwrap();
        // Tool message should be skipped
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, Some("user".to_string()));
    }

    #[test]
    fn test_convert_gemini_to_openai_response() {
        let client = create_test_client();
        let gemini_response = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContentResponse {
                    parts: vec![GeminiPartResponse {
                        text: "This is a test response".to_string(),
                    }],
                },
            }],
        };

        let result = client.convert_gemini_to_openai_response(gemini_response);
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
                            text: "First part".to_string(),
                        },
                        GeminiPartResponse {
                            text: "Second part".to_string(),
                        },
                    ],
                },
            }],
        };

        let result = client.convert_gemini_to_openai_response(gemini_response);
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
        };

        let result = client.convert_gemini_to_openai_response(gemini_response);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no candidates"));
    }
}
