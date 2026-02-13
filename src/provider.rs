use crate::models::{OllamaTagsResponse, Provider, ProviderInfo};
use anyhow::{anyhow, Result};
use std::env;

/// Check if a model supports function/tool calling
pub fn supports_tools(provider: &Provider, model: &str) -> bool {
    match provider {
        // All modern OpenAI models support tools
        Provider::OpenAI => true,

        // All Gemini models we support have tool capability
        Provider::Gemini => true,

        // All Claude models support tools
        Provider::Anthropic => true,

        // Azure OpenAI models support tools (same as OpenAI)
        Provider::AzureOpenAI => true,

        // Ollama: check by model family
        Provider::Ollama => {
            let model_lower = model.to_lowercase();

            // Known tool-supporting model families
            let tool_families = [
                "llama3.1", "llama3.2", "llama3.3",
                "qwen2", "qwen2.5", "qwen3",
                "mistral-nemo", "mistral-large",
                "command-r",
                "granite",
                "hermes",
                "deepseek",
                "glm",
            ];

            tool_families.iter().any(|f| model_lower.contains(f))
        }
    }
}

/// Check if Ollama is available and optionally if a specific model exists
pub fn check_ollama_available(model: Option<&str>) -> Result<Vec<String>> {
    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let url = format!("{}/api/tags", ollama_host);

    let response = reqwest::blocking::get(&url);
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let tags: OllamaTagsResponse = resp.json()?;
                let models: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();

                if let Some(target_model) = model {
                    if models.iter().any(|m| m == target_model || m.starts_with(&format!("{}:", target_model))) {
                        Ok(models)
                    } else {
                        Err(anyhow!("Model '{}' not found in Ollama", target_model))
                    }
                } else {
                    Ok(models)
                }
            } else {
                Err(anyhow!("Ollama returned error status: {}", resp.status()))
            }
        }
        Err(_) => Err(anyhow!("Ollama not available at {}", ollama_host)),
    }
}

/// Resolve Anthropic model aliases to full model names
fn resolve_anthropic_alias(model: &str) -> &str {
    match model {
        "sonnet" | "claude-sonnet" => "claude-sonnet-4-20250514",
        "sonnet-4.5" => "claude-sonnet-4-5-20250929",
        "opus" | "claude-opus" => "claude-opus-4-5-20251101", // Updated to Opus 4.5
        "opus-4.5" => "claude-opus-4-5-20251101",
        "opus-4.1" => "claude-opus-4-1-20250805",
        "haiku" | "claude-haiku" => "claude-haiku-4-5-20251001",
        "haiku-4.5" => "claude-haiku-4-5-20251001",
        _ => model,
    }
}

/// Resolve Gemini model aliases to full model names
fn resolve_gemini_alias(model: &str) -> &str {
    match model {
        "flash" | "gemini-3-flash" => "gemini-3-flash-preview",
        "pro" | "gemini-3-pro" => "gemini-3-pro-preview",
        _ => model,
    }
}

/// Detect the provider based on model name
pub fn detect_provider(model: &str) -> Result<ProviderInfo> {
    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());

    // 1. Check for Gemini models (explicit prefix or aliases)
    if model.starts_with("gemini")
        || model == "gemini-3-flash"
        || model == "gemini-3-pro"
        || model == "flash"
        || model == "pro"
    {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("GEMINI_API_KEY required for model '{}'", model))?;

        // Resolve aliases (gemini-3-flash -> gemini-3-flash-preview, etc.)
        let resolved_model = resolve_gemini_alias(model).to_string();

        // Check if this is a Gemini 3 model which uses native API
        let use_native_api = resolved_model == "gemini-3-pro-preview"
            || resolved_model == "gemini-3-flash-preview";
        let base_url = if use_native_api {
            "https://generativelanguage.googleapis.com/v1beta/models/".to_string()
        } else {
            "https://generativelanguage.googleapis.com/v1beta/openai/".to_string()
        };

        return Ok(ProviderInfo {
            provider: Provider::Gemini,
            base_url,
            api_key,
            resolved_model,
            use_native_gemini_api: use_native_api,
            azure_api_version: None,
        });
    }

    // 2. Check for Anthropic/Claude models (explicit prefix/names)
    if model.starts_with("claude")
        || model == "sonnet"
        || model == "sonnet-4.5"
        || model == "opus"
        || model == "opus-4.5"
        || model == "opus-4.1"
        || model == "haiku"
        || model == "haiku-4.5"
    {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow!("ANTHROPIC_API_KEY required for model '{}'", model))?;

        let resolved_model = resolve_anthropic_alias(model).to_string();

        return Ok(ProviderInfo {
            provider: Provider::Anthropic,
            base_url: "https://api.anthropic.com/v1/".to_string(),
            api_key,
            resolved_model,
            use_native_gemini_api: false,
            azure_api_version: None,
        });
    }

    // 3. Check for Azure OpenAI models (azure:<deployment-name>)
    if let Some(deployment) = model.strip_prefix("azure:") {
        let endpoint = env::var("AZURE_OPENAI_ENDPOINT")
            .map_err(|_| anyhow!("AZURE_OPENAI_ENDPOINT required for Azure OpenAI models"))?;
        let api_key = env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| anyhow!("AZURE_OPENAI_API_KEY required for Azure OpenAI models"))?;
        let api_version = env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-02-01".to_string());

        // Normalize endpoint (remove trailing slash if present)
        let endpoint = endpoint.trim_end_matches('/');

        return Ok(ProviderInfo {
            provider: Provider::AzureOpenAI,
            base_url: format!("{}/openai/deployments/", endpoint),
            api_key,
            resolved_model: deployment.to_string(),
            use_native_gemini_api: false,
            azure_api_version: Some(api_version),
        });
    }

    // 4. Check if the model exists in Ollama (even if it matches OpenAI patterns)
    // This allows local models like gpt-oss to be routed to Ollama
    if check_ollama_available(Some(model)).is_ok() {
        return Ok(ProviderInfo {
            provider: Provider::Ollama,
            base_url: format!("{}/v1/", ollama_host),
            api_key: "ollama".to_string(),
            resolved_model: model.to_string(),
            use_native_gemini_api: false,
            azure_api_version: None,
        });
    }

    // 5. Check for explicit OpenAI patterns
    let is_openai_pattern = model.starts_with("gpt-")
        || model.starts_with("gpt4")
        || model.starts_with("gpt5")
        || model.starts_with("chatgpt")
        || model == "o1"
        || model == "o1-mini"
        || model == "o1-preview"
        || model == "o3"
        || model == "o3-mini";

    if is_openai_pattern {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("OPENAI_API_KEY required for model '{}'", model))?;

        return Ok(ProviderInfo {
            provider: Provider::OpenAI,
            base_url: "https://api.openai.com/v1/".to_string(),
            api_key,
            resolved_model: model.to_string(),
            use_native_gemini_api: false,
            azure_api_version: None,
        });
    }

    // 6. Fallback: assume Ollama but warn if not available
    if check_ollama_available(None).is_ok() {
        Ok(ProviderInfo {
            provider: Provider::Ollama,
            base_url: format!("{}/v1/", ollama_host),
            api_key: "ollama".to_string(),
            resolved_model: model.to_string(),
            use_native_gemini_api: false,
            azure_api_version: None,
        })
    } else {
        Err(anyhow!(
            "Unknown model '{}' and Ollama is not available. \
            Please specify a valid model or ensure Ollama is running.",
            model
        ))
    }
}

/// Get the smart default model based on available providers
pub fn get_smart_default_model() -> Result<String> {
    // 1. Try Gemini first (preferred default) - use gemini-3-flash-preview for speed
    if env::var("GEMINI_API_KEY").is_ok() {
        return Ok("gemini-3-flash-preview".to_string());
    }

    // 2. Try Anthropic
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        return Ok("sonnet".to_string());
    }

    // 3. Try OpenAI
    if env::var("OPENAI_API_KEY").is_ok() {
        return Ok("gpt-5.1".to_string());
    }

    // 4. Try Ollama (local models)
    if let Ok(models) = check_ollama_available(None) {
        // Preferred Ollama models in order
        let preferred = ["llama3.1:latest", "deepseek-r1:latest", "gpt-oss:latest"];

        for pref in &preferred {
            if models.iter().any(|m| m == *pref) {
                return Ok(pref.to_string());
            }
        }

        // Use first available model
        if let Some(first) = models.first() {
            return Ok(first.clone());
        }
    }

    // 5. No providers available
    Err(anyhow!(
        "No AI providers available. Please either:\n\
        - Set GEMINI_API_KEY for Google Gemini\n\
        - Set ANTHROPIC_API_KEY for Claude\n\
        - Set OPENAI_API_KEY for OpenAI\n\
        - Install and run Ollama with at least one model"
    ))
}

/// Get list of all available models grouped by provider
pub fn get_available_models() -> Vec<(Provider, Vec<String>, bool)> {
    let mut result = Vec::new();

    // OpenAI
    let openai_models = vec![
        "gpt-5.1".to_string(),
        "gpt-5.1-codex".to_string(),
        "gpt-5.1-codex-mini".to_string(),
        "gpt-5.1-codex-max".to_string(),
        "gpt-4o".to_string(),
        "gpt-4-turbo".to_string(),
        "o1".to_string(),
        "o3".to_string(),
        "o3-mini".to_string(),
    ];
    let openai_available = env::var("OPENAI_API_KEY").is_ok();
    result.push((Provider::OpenAI, openai_models, openai_available));

    // Gemini
    let gemini_models = vec![
        "gemini-3-flash, gemini-3-flash-preview (default)".to_string(),
        "gemini-3-pro, gemini-3-pro-preview".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-flash-lite".to_string(),
        "gemini-2.5-pro".to_string(),
        "gemini-1.5-flash".to_string(),
        "gemini-1.5-pro".to_string(),
    ];
    let gemini_available = env::var("GEMINI_API_KEY").is_ok();
    result.push((Provider::Gemini, gemini_models, gemini_available));

    // Anthropic
    let anthropic_models = vec![
        "opus, opus-4.5 (claude-opus-4-5-20251101)".to_string(),
        "sonnet (claude-sonnet-4-20250514)".to_string(),
        "sonnet-4.5 (claude-sonnet-4-5-20250929)".to_string(),
        "opus-4.1 (claude-opus-4-1-20250805)".to_string(),
        "haiku, haiku-4.5 (claude-haiku-4-5-20251001)".to_string(),
    ];
    let anthropic_available = env::var("ANTHROPIC_API_KEY").is_ok();
    result.push((Provider::Anthropic, anthropic_models, anthropic_available));

    // Azure OpenAI
    let azure_available = env::var("AZURE_OPENAI_ENDPOINT").is_ok() && env::var("AZURE_OPENAI_API_KEY").is_ok();
    let azure_models = if azure_available {
        vec!["azure:<deployment-name> (use your Azure deployment name)".to_string()]
    } else {
        vec![]
    };
    result.push((Provider::AzureOpenAI, azure_models, azure_available));

    // Ollama
    let ollama_models = check_ollama_available(None).unwrap_or_default();
    let ollama_available = !ollama_models.is_empty();
    result.push((Provider::Ollama, ollama_models, ollama_available));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_alias_resolution() {
        assert_eq!(resolve_anthropic_alias("sonnet"), "claude-sonnet-4-20250514");
        assert_eq!(resolve_anthropic_alias("opus"), "claude-opus-4-5-20251101");
        assert_eq!(resolve_anthropic_alias("opus-4.5"), "claude-opus-4-5-20251101");
        assert_eq!(resolve_anthropic_alias("opus-4.1"), "claude-opus-4-1-20250805");
        assert_eq!(resolve_anthropic_alias("haiku"), "claude-haiku-4-5-20251001");
        assert_eq!(resolve_anthropic_alias("sonnet-4.5"), "claude-sonnet-4-5-20250929");
        assert_eq!(resolve_anthropic_alias("haiku-4.5"), "claude-haiku-4-5-20251001");
        // Pass through if not an alias
        assert_eq!(resolve_anthropic_alias("claude-sonnet-4-20250514"), "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_gemini_3_pro_preview_uses_native_api() {
        // Set a dummy API key for testing
        std::env::set_var("GEMINI_API_KEY", "test-key");

        let result = detect_provider("gemini-3-pro-preview");
        assert!(result.is_ok());

        let provider_info = result.unwrap();
        assert_eq!(provider_info.provider, Provider::Gemini);
        assert!(provider_info.use_native_gemini_api);
        assert_eq!(provider_info.base_url, "https://generativelanguage.googleapis.com/v1beta/models/");
        assert_eq!(provider_info.resolved_model, "gemini-3-pro-preview");

        // Clean up
        std::env::remove_var("GEMINI_API_KEY");
    }

    #[test]
    fn test_gemini_3_flash_preview_uses_native_api() {
        std::env::set_var("GEMINI_API_KEY", "test-key");

        let result = detect_provider("gemini-3-flash-preview");
        assert!(result.is_ok());

        let provider_info = result.unwrap();
        assert_eq!(provider_info.provider, Provider::Gemini);
        assert!(provider_info.use_native_gemini_api);
        assert_eq!(provider_info.base_url, "https://generativelanguage.googleapis.com/v1beta/models/");
        assert_eq!(provider_info.resolved_model, "gemini-3-flash-preview");

        std::env::remove_var("GEMINI_API_KEY");
    }

    #[test]
    fn test_gemini_alias_resolution() {
        assert_eq!(resolve_gemini_alias("gemini-3-flash"), "gemini-3-flash-preview");
        assert_eq!(resolve_gemini_alias("gemini-3-pro"), "gemini-3-pro-preview");
        // Short aliases
        assert_eq!(resolve_gemini_alias("flash"), "gemini-3-flash-preview");
        assert_eq!(resolve_gemini_alias("pro"), "gemini-3-pro-preview");
        // Pass through if not an alias
        assert_eq!(resolve_gemini_alias("gemini-2.5-flash"), "gemini-2.5-flash");
    }

    #[test]
    fn test_gemini_alias_uses_native_api() {
        std::env::set_var("GEMINI_API_KEY", "test-key");

        // Test gemini-3-flash alias
        let result = detect_provider("gemini-3-flash");
        assert!(result.is_ok());
        let provider_info = result.unwrap();
        assert!(provider_info.use_native_gemini_api);
        assert_eq!(provider_info.resolved_model, "gemini-3-flash-preview");

        // Test gemini-3-pro alias
        let result = detect_provider("gemini-3-pro");
        assert!(result.is_ok());
        let provider_info = result.unwrap();
        assert!(provider_info.use_native_gemini_api);
        assert_eq!(provider_info.resolved_model, "gemini-3-pro-preview");

        std::env::remove_var("GEMINI_API_KEY");
    }

    #[test]
    fn test_other_gemini_models_use_openai_compatible() {
        std::env::set_var("GEMINI_API_KEY", "test-key");

        let models = vec!["gemini-2.5-flash", "gemini-2.5-pro", "gemini-1.5-flash"];

        for model in models {
            let result = detect_provider(model);
            assert!(result.is_ok());

            let provider_info = result.unwrap();
            assert_eq!(provider_info.provider, Provider::Gemini);
            assert!(!provider_info.use_native_gemini_api);
            assert_eq!(provider_info.base_url, "https://generativelanguage.googleapis.com/v1beta/openai/");
        }

        std::env::remove_var("GEMINI_API_KEY");
    }

    #[test]
    fn test_anthropic_detection() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");

        let result = detect_provider("sonnet");
        assert!(result.is_ok());

        let provider_info = result.unwrap();
        assert_eq!(provider_info.provider, Provider::Anthropic);
        assert!(!provider_info.use_native_gemini_api);
        assert_eq!(provider_info.resolved_model, "claude-sonnet-4-20250514");

        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_openai_detection() {
        std::env::set_var("OPENAI_API_KEY", "test-key");

        let models = vec!["gpt-4o", "gpt-4", "o1", "o1-mini"];

        for model in models {
            let result = detect_provider(model);
            assert!(result.is_ok());

            let provider_info = result.unwrap();
            assert_eq!(provider_info.provider, Provider::OpenAI);
            assert!(!provider_info.use_native_gemini_api);
        }

        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn test_azure_openai_detection() {
        std::env::set_var("AZURE_OPENAI_ENDPOINT", "https://test.openai.azure.com");
        std::env::set_var("AZURE_OPENAI_API_KEY", "test-key");

        let result = detect_provider("azure:gpt-4o-mini");
        assert!(result.is_ok());

        let provider_info = result.unwrap();
        assert_eq!(provider_info.provider, Provider::AzureOpenAI);
        assert_eq!(provider_info.resolved_model, "gpt-4o-mini");
        assert_eq!(provider_info.base_url, "https://test.openai.azure.com/openai/deployments/");
        assert!(provider_info.azure_api_version.is_some());
        assert!(!provider_info.use_native_gemini_api);

        std::env::remove_var("AZURE_OPENAI_ENDPOINT");
        std::env::remove_var("AZURE_OPENAI_API_KEY");
    }

    #[test]
    fn test_azure_openai_requires_env_vars() {
        // Clear any existing env vars
        std::env::remove_var("AZURE_OPENAI_ENDPOINT");
        std::env::remove_var("AZURE_OPENAI_API_KEY");

        let result = detect_provider("azure:gpt-4o-mini");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("AZURE_OPENAI_ENDPOINT"));
    }

    #[test]
    fn test_azure_openai_custom_api_version() {
        std::env::set_var("AZURE_OPENAI_ENDPOINT", "https://test.openai.azure.com");
        std::env::set_var("AZURE_OPENAI_API_KEY", "test-key");
        std::env::set_var("AZURE_OPENAI_API_VERSION", "2024-08-01");

        let result = detect_provider("azure:my-deployment");
        assert!(result.is_ok());

        let provider_info = result.unwrap();
        assert_eq!(provider_info.azure_api_version, Some("2024-08-01".to_string()));

        std::env::remove_var("AZURE_OPENAI_ENDPOINT");
        std::env::remove_var("AZURE_OPENAI_API_KEY");
        std::env::remove_var("AZURE_OPENAI_API_VERSION");
    }
}
