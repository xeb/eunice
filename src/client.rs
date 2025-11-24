use crate::models::{
    ChatCompletionRequest, ChatCompletionResponse, Message, Provider, ProviderInfo, Tool,
};
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

/// OpenAI-compatible HTTP client for all providers
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    #[allow(dead_code)]
    api_key: String,
    provider: Provider,
}

impl Client {
    /// Create a new client for the given provider
    pub fn new(provider_info: &ProviderInfo) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Set authorization header based on provider
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
        })
    }

    /// Send a chat completion request
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatCompletionResponse> {
        let url = format!("{}chat/completions", self.base_url);

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            tool_choice: tools.map(|_| "auto".to_string()),
        };

        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        let response_body = response
            .json::<ChatCompletionResponse>()
            .await
            .context("Failed to parse response")?;

        Ok(response_body)
    }

    /// Get the provider type
    pub fn provider(&self) -> &Provider {
        &self.provider
    }
}
