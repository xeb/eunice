use crate::models::{OllamaTagsResponse, Provider, ProviderInfo};
use anyhow::{anyhow, Result};
use std::env;

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
        "opus" | "claude-opus" => "claude-opus-4-1-20250805",
        "haiku" | "claude-haiku" => "claude-haiku-4-5-20251001",
        "haiku-4.5" => "claude-haiku-4-5-20251001",
        _ => model,
    }
}

/// Detect the provider based on model name
pub fn detect_provider(model: &str) -> Result<ProviderInfo> {
    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());

    // 1. Check for Gemini models (explicit prefix)
    if model.starts_with("gemini") {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("GEMINI_API_KEY required for model '{}'", model))?;

        return Ok(ProviderInfo {
            provider: Provider::Gemini,
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai/".to_string(),
            api_key,
            resolved_model: model.to_string(),
        });
    }

    // 2. Check for Anthropic/Claude models (explicit prefix/names)
    if model.starts_with("claude")
        || model == "sonnet"
        || model == "sonnet-4.5"
        || model == "opus"
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
        });
    }

    // 3. Check if the model exists in Ollama (even if it matches OpenAI patterns)
    // This allows local models like gpt-oss to be routed to Ollama
    if check_ollama_available(Some(model)).is_ok() {
        return Ok(ProviderInfo {
            provider: Provider::Ollama,
            base_url: format!("{}/v1/", ollama_host),
            api_key: "ollama".to_string(),
            resolved_model: model.to_string(),
        });
    }

    // 4. Check for explicit OpenAI patterns
    let is_openai_pattern = model.starts_with("gpt-")
        || model.starts_with("gpt4")
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
        });
    }

    // 5. Fallback: assume Ollama but warn if not available
    if check_ollama_available(None).is_ok() {
        Ok(ProviderInfo {
            provider: Provider::Ollama,
            base_url: format!("{}/v1/", ollama_host),
            api_key: "ollama".to_string(),
            resolved_model: model.to_string(),
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
    // 1. Try Gemini first (preferred default)
    if env::var("GEMINI_API_KEY").is_ok() {
        return Ok("gemini-2.5-flash".to_string());
    }

    // 2. Try Anthropic
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        return Ok("sonnet".to_string());
    }

    // 3. Try OpenAI
    if env::var("OPENAI_API_KEY").is_ok() {
        return Ok("gpt-4o".to_string());
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
        "gpt-4o".to_string(),
        "gpt-4-turbo".to_string(),
        "gpt-4".to_string(),
        "gpt-3.5-turbo".to_string(),
        "o1".to_string(),
        "o1-mini".to_string(),
    ];
    let openai_available = env::var("OPENAI_API_KEY").is_ok();
    result.push((Provider::OpenAI, openai_models, openai_available));

    // Gemini
    let gemini_models = vec![
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
        "sonnet (claude-sonnet-4-20250514)".to_string(),
        "sonnet-4.5 (claude-sonnet-4-5-20250929)".to_string(),
        "opus (claude-opus-4-1-20250805)".to_string(),
        "haiku (claude-haiku-4-5-20251001)".to_string(),
    ];
    let anthropic_available = env::var("ANTHROPIC_API_KEY").is_ok();
    result.push((Provider::Anthropic, anthropic_models, anthropic_available));

    // Ollama
    let ollama_models = check_ollama_available(None).unwrap_or_default();
    let ollama_available = !ollama_models.is_empty();
    result.push((Provider::Ollama, ollama_models, ollama_available));

    result
}
