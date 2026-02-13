use crate::compact::{extract_retry_delay, is_rate_limit_error};
use crate::key_rotation::{is_bad_key_error, is_quota_error, BadKeyAction, KeyPool, RateLimitAction};
use crate::models::{
    ChatCompletionRequest, ChatCompletionResponse, GeminiContent, GeminiPart, GeminiRequest, GeminiTool,
    GeminiResponse, Message, Provider, ProviderInfo, Tool,
};
use anyhow::{anyhow, Context, Result};
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::sync::Arc;
use std::time::Duration;

/// Retry configuration for API requests
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
        }
    }
}

/// OpenAI-compatible HTTP client for all providers
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    key_pool: Arc<KeyPool>,
    provider: Provider,
    use_native_gemini_api: bool,
    retry_config: RetryConfig,
    /// Azure OpenAI API version (e.g., "2024-02-01")
    azure_api_version: Option<String>,
    /// Enable debug output
    debug: bool,
}

impl Client {
    /// Create a new client for the given provider
    pub fn new(provider_info: &ProviderInfo) -> Result<Self> {
        // Load key pool based on provider
        let key_pool = match provider_info.provider {
            Provider::Gemini => {
                // Try to load from key file, fall back to single key from ProviderInfo
                match KeyPool::load_gemini() {
                    Ok(pool) => Arc::new(pool),
                    Err(_) => Arc::new(KeyPool::single(provider_info.api_key.clone())),
                }
            }
            _ => Arc::new(KeyPool::single(provider_info.api_key.clone())),
        };

        Self::with_key_pool(provider_info, key_pool)
    }

    /// Create a new client with a specific key pool
    pub fn with_key_pool(provider_info: &ProviderInfo, key_pool: Arc<KeyPool>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Anthropic requires a version header on every request
        if let Provider::Anthropic = provider_info.provider {
            headers.insert(
                "anthropic-version",
                HeaderValue::from_static("2023-06-01"),
            );
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http,
            base_url: provider_info.base_url.clone(),
            key_pool,
            provider: provider_info.provider.clone(),
            use_native_gemini_api: provider_info.use_native_gemini_api,
            retry_config: RetryConfig::default(),
            azure_api_version: provider_info.azure_api_version.clone(),
            debug: std::env::var("EUNICE_DEBUG").is_ok(),
        })
    }

    /// Enable or disable debug mode
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Get the current API key
    fn current_api_key(&self) -> &str {
        self.key_pool.current_key()
    }

    /// Handle a rate limit error, returns action to take
    pub fn handle_rate_limit(&self) -> RateLimitAction {
        self.key_pool.handle_rate_limit()
    }

    /// Handle a bad key error, returns action to take
    pub fn handle_bad_key(&self) -> BadKeyAction {
        self.key_pool.handle_bad_key()
    }

    /// Check if an error is a rate limit error
    pub fn is_quota_error(error_msg: &str) -> bool {
        is_quota_error(error_msg)
    }

    /// Check if an error is a bad key error
    pub fn is_bad_key_error(error_msg: &str) -> bool {
        is_bad_key_error(error_msg)
    }

    /// Get key pool info for display
    pub fn key_info(&self) -> (usize, usize) {
        (self.key_pool.current_index_display(), self.key_pool.key_count())
    }

    /// Add auth header to a request based on provider
    fn add_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let api_key = self.current_api_key();
        match self.provider {
            Provider::Anthropic => req.header("x-api-key", api_key),
            Provider::Ollama => req, // No auth needed
            Provider::AzureOpenAI => req.header("api-key", api_key),
            _ => req.header(AUTHORIZATION, format!("Bearer {}", api_key)),
        }
    }

    /// Calculate backoff delay with jitter for a given attempt
    fn backoff_delay(&self, attempt: u32) -> Duration {
        let base = self.retry_config.initial_delay_ms * 2u64.pow(attempt.saturating_sub(1));
        let capped = base.min(self.retry_config.max_delay_ms);
        let jitter = rand::thread_rng().gen_range(0..=self.retry_config.initial_delay_ms);
        Duration::from_millis(capped + jitter)
    }

    /// Check if a status code is retryable
    fn is_retryable_status(status: u16) -> bool {
        status == 429 || status == 500 || status == 502 || status == 503
    }

    /// Send a chat completion request
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: serde_json::Value,
        tools: Option<&[Tool]>,
    ) -> Result<ChatCompletionResponse> {
        // Check if using native Gemini API
        if self.use_native_gemini_api {
            let messages: Vec<Message> = serde_json::from_value(messages)?;
            return self
                .chat_completion_gemini_native(model, &messages, tools)
                .await;
        }

        // Standard OpenAI-compatible API
        let url = if let Provider::AzureOpenAI = self.provider {
            // Azure OpenAI: {base_url}{deployment}/chat/completions?api-version={version}
            let api_version = self.azure_api_version.as_deref().unwrap_or("2024-02-01");
            format!("{}{}/chat/completions?api-version={}", self.base_url, model, api_version)
        } else {
            format!("{}chat/completions", self.base_url)
        };

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            tools: tools.map(|t| t.to_vec()),
            tool_choice: tools.map(|_| "auto".to_string()),
        };

        let mut attempt = 0u32;
        loop {
            if self.debug {
                eprintln!("[DEBUG] POST {} (attempt {})", url, attempt + 1);
                eprintln!("[DEBUG] Provider: {:?}, Model: {}", self.provider, model);
            }

            let start = std::time::Instant::now();
            let response = self
                .add_auth(self.http.post(&url))
                .json(&request)
                .send()
                .await
                .context("Failed to send request")?;

            let elapsed = start.elapsed();
            let status = response.status().as_u16();

            if self.debug {
                eprintln!("[DEBUG] Response: {} in {:.2}s", status, elapsed.as_secs_f64());
            }

            if Self::is_retryable_status(status) {
                let error_text = response.text().await.unwrap_or_default();

                if attempt >= self.retry_config.max_retries {
                    return Err(anyhow!(
                        "API request failed with status {} after {} retries: {}",
                        status,
                        attempt,
                        error_text
                    ));
                }

                // Calculate delay: use server-suggested delay if available, otherwise backoff
                let delay = if is_rate_limit_error(&error_text) {
                    if let Some(secs) = extract_retry_delay(&error_text) {
                        Duration::from_secs(secs)
                    } else {
                        self.backoff_delay(attempt)
                    }
                } else {
                    self.backoff_delay(attempt)
                };

                eprintln!(
                    "⏳ {} ({}): retrying in {:.1}s (attempt {}/{})...",
                    if status == 429 { "Rate limit" } else { "Server error" },
                    status,
                    delay.as_secs_f64(),
                    attempt + 1,
                    self.retry_config.max_retries
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }

            if !response.status().is_success() {
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
    ) -> Result<ChatCompletionResponse> {
        use crate::models::GeminiFunctionDeclaration;

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
                function_declarations: Some(declarations),
                code_execution: None,
            }]
        });

        let gemini_request = GeminiRequest {
            contents,
            tools: gemini_tools,
        };

        // Build URL: base_url already contains /v1beta/models/
        let url = format!("{}{}:generateContent", self.base_url, model);

        let mut attempt = 0u32;
        loop {
            if self.debug {
                eprintln!("[DEBUG] POST {} (Gemini native, attempt {})", url, attempt + 1);
            }

            let start = std::time::Instant::now();
            let response = self
                .http
                .post(&url)
                .header("x-goog-api-key", self.current_api_key())
                .header(CONTENT_TYPE, "application/json")
                .json(&gemini_request)
                .send()
                .await
                .context("Failed to send Gemini request")?;

            let elapsed = start.elapsed();
            let status = response.status().as_u16();

            if self.debug {
                eprintln!("[DEBUG] Response: {} in {:.2}s", status, elapsed.as_secs_f64());
            }

            if Self::is_retryable_status(status) {
                let error_text = response.text().await.unwrap_or_default();

                if attempt >= self.retry_config.max_retries {
                    return Err(anyhow!(
                        "Gemini API request failed with status {} after {} retries: {}",
                        status,
                        attempt,
                        error_text
                    ));
                }

                // Calculate delay
                let delay = if is_rate_limit_error(&error_text) {
                    if let Some(secs) = extract_retry_delay(&error_text) {
                        Duration::from_secs(secs)
                    } else {
                        self.backoff_delay(attempt)
                    }
                } else {
                    self.backoff_delay(attempt)
                };

                eprintln!(
                    "⏳ {} ({}): retrying in {:.1}s (attempt {}/{})...",
                    if status == 429 { "Rate limit" } else { "Server error" },
                    status,
                    delay.as_secs_f64(),
                    attempt + 1,
                    self.retry_config.max_retries
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }

            if !response.status().is_success() {
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

    /// Send a streaming chat completion request using native Gemini API
    /// Calls the callback for each text chunk as it arrives
    /// Returns the complete response with all function calls
    pub async fn chat_completion_streaming<F>(
        &self,
        model: &str,
        messages: serde_json::Value,
        tools: Option<&[Tool]>,
        mut on_chunk: F,
    ) -> Result<ChatCompletionResponse>
    where
        F: FnMut(&str),
    {
        // Only streaming for native Gemini API
        if !self.use_native_gemini_api {
            // Fall back to non-streaming for other providers
            return self.chat_completion(model, messages, tools).await;
        }

        use crate::models::GeminiFunctionDeclaration;
        use futures::StreamExt;

        let messages: Vec<Message> = serde_json::from_value(messages)?;
        let contents = self.convert_messages_to_gemini(&messages)?;

        // Convert tools to Gemini format
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
                function_declarations: Some(declarations),
                code_execution: None,
            }]
        });

        let gemini_request = GeminiRequest {
            contents,
            tools: gemini_tools,
        };

        // Use streamGenerateContent endpoint
        let url = format!("{}{}:streamGenerateContent?alt=sse", self.base_url, model);

        if self.debug {
            eprintln!("[DEBUG] POST {} (streaming)", url);
            eprintln!("[DEBUG] Provider: {:?}, Model: {}", self.provider, model);
        }

        let start = std::time::Instant::now();
        let response = self
            .http
            .post(&url)
            .header("x-goog-api-key", self.current_api_key())
            .header(CONTENT_TYPE, "application/json")
            .json(&gemini_request)
            .send()
            .await
            .context("Failed to send Gemini streaming request")?;

        if self.debug {
            let elapsed = start.elapsed();
            eprintln!("[DEBUG] Streaming response started: {} in {:.2}s", response.status(), elapsed.as_secs_f64());
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Gemini streaming request failed with status {}: {}",
                status,
                error_text
            ));
        }

        // Collect all text and function calls from the stream
        let mut all_text = String::new();
        let mut all_tool_calls: Vec<crate::models::ToolCall> = Vec::new();
        let mut usage_metadata: Option<crate::models::GeminiUsageMetadata> = None;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8_lossy(&chunk);

            buffer.push_str(&chunk_str);

            // Process complete SSE events in the buffer
            // SSE format: "data: {json}\n\n" or "data: {json}\r\n\r\n"
            loop {
                // Find "data: " prefix
                let Some(data_start) = buffer.find("data: ") else {
                    // No more data: prefixes, but check for raw JSON (error responses)
                    if buffer.trim_start().starts_with('{') {
                        // Try to parse as error or raw JSON - just clear the buffer
                        if serde_json::from_str::<serde_json::Value>(buffer.trim()).is_ok() {
                            buffer.clear();
                        }
                    }
                    break;
                };

                let json_start = data_start + 6;

                // Find the end of the JSON (either \n\n, \r\n\r\n, or another "data: ")
                let json_end = buffer[json_start..]
                    .find("\n\ndata: ")
                    .or_else(|| buffer[json_start..].find("\r\n\r\n"))
                    .or_else(|| buffer[json_start..].find("\n\n"))
                    .map(|i| json_start + i);

                let json_end = match json_end {
                    Some(end) => end,
                    None => {
                        // No clear end found - try parsing what we have
                        // This handles the case where we're at the end of the stream
                        if buffer.ends_with("\n") || buffer.ends_with("\r\n") {
                            buffer.len()
                        } else {
                            break; // Wait for more data
                        }
                    }
                };

                let json_str = buffer[json_start..json_end].trim();

                // Parse the JSON chunk
                if let Ok(chunk_response) = serde_json::from_str::<GeminiResponse>(json_str) {
                    // Extract text and stream it
                    if !chunk_response.candidates.is_empty() {
                        let candidate = &chunk_response.candidates[0];
                        for part in &candidate.content.parts {
                            if let Some(ref text) = part.text {
                                on_chunk(text);
                                all_text.push_str(text);
                            }
                            // Collect function calls
                            if let Some(ref fc) = part.function_call {
                                let id = match &part.thought_signature {
                                    Some(sig) => format!("{}::{}", fc.name, sig),
                                    None => fc.name.clone(),
                                };
                                all_tool_calls.push(crate::models::ToolCall {
                                    id,
                                    call_type: "function".to_string(),
                                    function: crate::models::FunctionCall {
                                        name: fc.name.clone(),
                                        arguments: fc.args.to_string(),
                                    },
                                });
                            }
                        }
                    }
                    // Capture usage metadata from last chunk
                    if chunk_response.usage_metadata.is_some() {
                        usage_metadata = chunk_response.usage_metadata;
                    }
                }

                // Remove processed data from buffer
                buffer = buffer[json_end..].trim_start_matches(['\n', '\r']).to_string();
            }
        }

        // Build the final response
        let tool_calls = if all_tool_calls.is_empty() {
            None
        } else {
            Some(all_tool_calls)
        };

        Ok(ChatCompletionResponse {
            choices: vec![crate::models::Choice {
                message: crate::models::AssistantMessage {
                    content: if all_text.is_empty() { None } else { Some(all_text) },
                    tool_calls,
                },
            }],
            usage: usage_metadata.map(|u| crate::models::UsageStats {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
                cached_tokens: u.cached_content_token_count,
            }),
        })
    }

    /// Check if streaming is supported for the current provider
    pub fn supports_streaming(&self) -> bool {
        self.use_native_gemini_api
    }

    /// Send a chat completion request with an image
    #[allow(dead_code)]
    pub async fn chat_completion_with_image(
        &self,
        model: &str,
        prompt: &str,
        image_data: &str, // base64
        mime_type: &str,
    ) -> Result<ChatCompletionResponse> {
        // Use native Gemini API for multimodal requests if configured
        if self.use_native_gemini_api {
            let gemini_request = GeminiRequest {
                contents: vec![GeminiContent {
                    parts: vec![
                        GeminiPart {
                            text: Some(prompt.to_string()),
                            inline_data: None,
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        },
                        GeminiPart {
                            text: None,
                            inline_data: Some(crate::models::GeminiInlineData {
                                mime_type: mime_type.to_string(),
                                data: image_data.to_string(),
                            }),
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        },
                    ],
                    role: Some("user".to_string()),
                }],
                // Enable Agentic Vision code execution for Gemini image analysis
                // This allows the model to zoom, crop, annotate, and analyze images with Python code
                tools: Some(vec![GeminiTool {
                    function_declarations: None,
                    code_execution: Some(serde_json::json!({})),
                }]),
            };

            let url = format!("{}{}:generateContent", self.base_url, model);
            let response = self
                .http
                .post(&url)
                .header("x-goog-api-key", self.current_api_key())
                .json(&gemini_request)
                .send()
                .await
                .context("Failed to send Gemini multimodal request")?;

            if !response.status().is_success() {
                return Err(anyhow!(
                    "Gemini API request failed with status {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                ));
            }

            let response_text = response.text().await.context("Failed to read Gemini response body")?;
            let gemini_response: GeminiResponse =
                serde_json::from_str(&response_text).context("Failed to parse Gemini response")?;
            return self.convert_gemini_to_openai_response(gemini_response, &response_text);
        }

        // Standard OpenAI-compatible API for multimodal
        let url = format!("{}chat/completions", self.base_url);
        let messages = serde_json::json!([
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{};base64,{}", mime_type, image_data)
                        }
                    }
                ]
            }
        ]);

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            tools: None,
            tool_choice: None,
        };

        let response = self
            .add_auth(self.http.post(&url))
            .json(&request)
            .send()
            .await
            .context("Failed to send OpenAI multimodal request")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "API request failed with status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        response
            .json::<ChatCompletionResponse>()
            .await
            .context("Failed to parse OpenAI multimodal response")
    }

    /// Convert OpenAI-style messages to Gemini contents format
    /// Groups consecutive Tool messages into a single content block for parallel function calling
    fn convert_messages_to_gemini(&self, messages: &[Message]) -> Result<Vec<GeminiContent>> {
        use crate::models::{GeminiFunctionCallRequest, GeminiFunctionResponse};

        let mut contents = Vec::new();
        let mut pending_tool_parts: Vec<GeminiPart> = Vec::new();

        // Helper to flush pending tool parts
        let flush_tool_parts = |contents: &mut Vec<GeminiContent>, parts: &mut Vec<GeminiPart>| {
            if !parts.is_empty() {
                contents.push(GeminiContent {
                    parts: std::mem::take(parts),
                    role: Some("user".to_string()),
                });
            }
        };

        for message in messages {
            match message {
                Message::User { content } => {
                    // Flush any pending tool responses before user message
                    flush_tool_parts(&mut contents, &mut pending_tool_parts);

                    contents.push(GeminiContent {
                        parts: vec![GeminiPart {
                            text: Some(content.clone()),
                            inline_data: None,
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        }],
                        role: Some("user".to_string()),
                    });
                }
                Message::Assistant { content, tool_calls } => {
                    // Flush any pending tool responses before assistant message
                    flush_tool_parts(&mut contents, &mut pending_tool_parts);

                    let mut parts = Vec::new();

                    // Add text content if present
                    if let Some(text) = content {
                        if !text.is_empty() {
                            parts.push(GeminiPart {
                                text: Some(text.clone()),
                                inline_data: None,
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
                                inline_data: None,
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
                    // Try to parse the content as JSON
                    let parsed = serde_json::from_str::<serde_json::Value>(content);

                    // Gemini requires functionResponse.response to be a Map (JSON object).
                    // If the tool output is a primitive or array, we must wrap it.
                    let response_value = match parsed {
                        Ok(val) if val.is_object() => val,
                        Ok(val) => serde_json::json!({ "result": val }),
                        Err(_) => serde_json::json!({ "result": content }),
                    };

                    // Extract function name and thought_signature from tool_call_id
                    // The ID may be encoded as "name::signature" or just "name"
                    let function_name = tool_call_id.split("::").next().unwrap_or(tool_call_id).to_string();
                    let thought_signature = if tool_call_id.contains("::") {
                        tool_call_id.split("::").nth(1).map(|s| s.to_string())
                    } else {
                        None
                    };

                    // Group consecutive tool responses into pending_tool_parts
                    pending_tool_parts.push(GeminiPart {
                        text: None,
                        inline_data: None,
                        function_call: None,
                        function_response: Some(GeminiFunctionResponse {
                            name: function_name,
                            response: response_value,
                        }),
                        thought_signature,
                    });
                }
            }
        }

        // Flush any remaining tool parts at the end
        flush_tool_parts(&mut contents, &mut pending_tool_parts);

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

        // Extract text content, including code execution results for Agentic Vision
        let mut text_parts: Vec<String> = Vec::new();
        for part in &candidate.content.parts {
            if let Some(ref t) = part.text {
                text_parts.push(t.clone());
            }
            // Include code execution results in the text output
            if let Some(ref code) = part.executable_code {
                text_parts.push(format!("\n```{}\n{}\n```", code.language.to_lowercase(), code.code));
            }
            if let Some(ref result) = part.code_execution_result {
                if let Some(ref output) = result.output {
                    text_parts.push(format!("\n**Code Output ({}):**\n```\n{}\n```", result.outcome, output));
                }
            }
        }
        let text = text_parts.join("\n");

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
            usage: gemini_response.usage_metadata.map(|u| crate::models::UsageStats {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
                cached_tokens: u.cached_content_token_count,
            }),
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
    use crate::models::{
        GeminiCandidate, GeminiCodeExecutionResult, GeminiContentResponse,
        GeminiExecutableCode, GeminiPartResponse,
    };

    fn create_test_client() -> Client {
        let provider_info = ProviderInfo {
            provider: Provider::Gemini,
            base_url: "https://test.com/".to_string(),
            api_key: "test-key".to_string(),
            resolved_model: "gemini-3-pro-preview".to_string(),
            use_native_gemini_api: true,
            azure_api_version: None,
        };
        Client::new(&provider_info).unwrap()
    }

    #[test]
    fn test_convert_messages_to_gemini_wraps_array_response() {
        let client = create_test_client();
        let messages = vec![
            Message::Tool {
                tool_call_id: "my_function".to_string(),
                content: "[\"item1\", \"item2\"]".to_string(),
            },
        ];

        let result = client.convert_messages_to_gemini(&messages);
        assert!(result.is_ok());
        let contents = result.unwrap();
        
        let response = contents[0].parts[0].function_response.as_ref().unwrap();
        // Check that it's wrapped in an object
        assert!(response.response.is_object());
        // Check content
        assert_eq!(response.response["result"][0], "item1");
    }

    #[test]
    fn test_convert_messages_to_gemini_preserves_object_response() {
        let client = create_test_client();
        let messages = vec![
            Message::Tool {
                tool_call_id: "my_function".to_string(),
                content: "{\"key\": \"value\"}".to_string(),
            },
        ];

        let result = client.convert_messages_to_gemini(&messages);
        assert!(result.is_ok());
        let contents = result.unwrap();
        
        let response = contents[0].parts[0].function_response.as_ref().unwrap();
        // Check that it's NOT wrapped
        assert_eq!(response.response["key"], "value");
        assert!(response.response.get("result").is_none());
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
                        executable_code: None,
                        code_execution_result: None,
                    }],
                },
                finish_reason: Some("STOP".to_string()),
                finish_message: None,
            }],
            prompt_feedback: None,
            usage_metadata: None,
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
                            executable_code: None,
                            code_execution_result: None,
                        },
                        GeminiPartResponse {
                            text: Some("Second part".to_string()),
                            function_call: None,
                            thought_signature: None,
                            executable_code: None,
                            code_execution_result: None,
                        },
                    ],
                },
                finish_reason: Some("STOP".to_string()),
                finish_message: None,
            }],
            prompt_feedback: None,
            usage_metadata: None,
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
            usage_metadata: None,
        };

        let result = client.convert_gemini_to_openai_response(gemini_response, "{}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no candidates"));
    }

    #[test]
    fn test_thought_signature_on_function_response() {
        let client = create_test_client();
        let messages = vec![
            Message::Tool {
                tool_call_id: "my_function::abc123sig".to_string(),
                content: "{\"status\": \"ok\"}".to_string(),
            },
        ];

        let result = client.convert_messages_to_gemini(&messages).unwrap();
        assert_eq!(result.len(), 1);

        let part = &result[0].parts[0];
        // Verify function_response has the correct name (without signature)
        let fr = part.function_response.as_ref().unwrap();
        assert_eq!(fr.name, "my_function");

        // Verify thought_signature is extracted
        assert_eq!(part.thought_signature, Some("abc123sig".to_string()));
    }

    #[test]
    fn test_thought_signature_absent_on_function_response() {
        let client = create_test_client();
        let messages = vec![
            Message::Tool {
                tool_call_id: "my_function".to_string(),
                content: "{\"value\": 42}".to_string(),
            },
        ];

        let result = client.convert_messages_to_gemini(&messages).unwrap();
        let part = &result[0].parts[0];

        let fr = part.function_response.as_ref().unwrap();
        assert_eq!(fr.name, "my_function");
        assert_eq!(part.thought_signature, None);
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 60000);
    }

    #[test]
    fn test_is_retryable_status() {
        assert!(Client::is_retryable_status(429));
        assert!(Client::is_retryable_status(500));
        assert!(Client::is_retryable_status(502));
        assert!(Client::is_retryable_status(503));
        assert!(!Client::is_retryable_status(200));
        assert!(!Client::is_retryable_status(400));
        assert!(!Client::is_retryable_status(401));
        assert!(!Client::is_retryable_status(404));
    }

    #[test]
    fn test_convert_gemini_to_openai_response_with_code_execution() {
        // Test Agentic Vision response parsing with executableCode and codeExecutionResult
        let client = create_test_client();
        let gemini_response = GeminiResponse {
            candidates: vec![GeminiCandidate {
                content: GeminiContentResponse {
                    parts: vec![
                        GeminiPartResponse {
                            text: Some("Let me analyze the image.".to_string()),
                            function_call: None,
                            thought_signature: None,
                            executable_code: None,
                            code_execution_result: None,
                        },
                        GeminiPartResponse {
                            text: None,
                            function_call: None,
                            thought_signature: None,
                            executable_code: Some(GeminiExecutableCode {
                                language: "PYTHON".to_string(),
                                code: "print('Hello from Agentic Vision')".to_string(),
                            }),
                            code_execution_result: None,
                        },
                        GeminiPartResponse {
                            text: None,
                            function_call: None,
                            thought_signature: None,
                            executable_code: None,
                            code_execution_result: Some(GeminiCodeExecutionResult {
                                outcome: "OUTCOME_OK".to_string(),
                                output: Some("Hello from Agentic Vision".to_string()),
                            }),
                        },
                        GeminiPartResponse {
                            text: Some("The analysis is complete.".to_string()),
                            function_call: None,
                            thought_signature: None,
                            executable_code: None,
                            code_execution_result: None,
                        },
                    ],
                },
                finish_reason: Some("STOP".to_string()),
                finish_message: None,
            }],
            prompt_feedback: None,
            usage_metadata: None,
        };

        let result = client.convert_gemini_to_openai_response(gemini_response, "{}");
        assert!(result.is_ok());

        let openai_response = result.unwrap();
        let content = openai_response.choices[0].message.content.as_ref().unwrap();

        // Verify all parts are included in the response
        assert!(content.contains("Let me analyze the image."));
        assert!(content.contains("```python"));
        assert!(content.contains("print('Hello from Agentic Vision')"));
        assert!(content.contains("OUTCOME_OK"));
        assert!(content.contains("Hello from Agentic Vision"));
        assert!(content.contains("The analysis is complete."));
    }
}
