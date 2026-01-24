//! API key rotation for multi-key configurations
//!
//! Supports loading multiple API keys per provider from `~/.eunice/api_keys.toml`
//! and rotating through them on rate limit errors.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// TOML configuration for API keys
#[derive(Debug, Deserialize, Default)]
pub struct ApiKeysConfig {
    pub gemini: Option<ProviderKeys>,
    pub anthropic: Option<ProviderKeys>,
    pub openai: Option<ProviderKeys>,
    pub ollama: Option<ProviderKeys>,
}

/// Keys for a single provider
#[derive(Debug, Deserialize, Clone)]
pub struct ProviderKeys {
    pub keys: Vec<String>,
}

/// Thread-safe API key rotator using atomic index
pub struct ApiKeyRotator {
    keys: Vec<String>,
    index: AtomicUsize,
}

impl ApiKeyRotator {
    /// Create a new rotator from a list of keys
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            keys,
            index: AtomicUsize::new(0),
        }
    }

    /// Get the current key
    pub fn current_key(&self) -> &str {
        let idx = self.index.load(Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }

    /// Rotate to the next key and return it.
    /// Returns None if we've cycled through all keys (back to start).
    pub fn rotate(&self) -> Option<&str> {
        if self.keys.len() <= 1 {
            return None; // Can't rotate with 0 or 1 key
        }
        let old = self.index.fetch_add(1, Ordering::Relaxed);
        let new_idx = (old + 1) % self.keys.len();
        // If we've cycled back to 0, all keys exhausted
        if new_idx == 0 {
            return None;
        }
        Some(&self.keys[new_idx])
    }

    /// Reset the rotation index (call after a successful request)
    pub fn reset(&self) {
        self.index.store(0, Ordering::Relaxed);
    }

    /// Number of keys available
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
}

/// Load API keys configuration from a TOML file
pub fn load_api_keys(path: &Path) -> Result<ApiKeysConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read API keys file: {}", path.display()))?;
    let config: ApiKeysConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse API keys file: {}", path.display()))?;
    Ok(config)
}

/// Build a rotator for a specific provider, combining env var key (first) with file keys
pub fn build_rotator(
    config: &ApiKeysConfig,
    provider: &crate::models::Provider,
    env_key: &str,
) -> Option<ApiKeyRotator> {
    let mut keys = Vec::new();

    // Env var key goes first (it's the "primary" key)
    if !env_key.is_empty() {
        keys.push(env_key.to_string());
    }

    // Add keys from config file
    let provider_keys = match provider {
        crate::models::Provider::Gemini => config.gemini.as_ref(),
        crate::models::Provider::Anthropic => config.anthropic.as_ref(),
        crate::models::Provider::OpenAI => config.openai.as_ref(),
        crate::models::Provider::Ollama => config.ollama.as_ref(),
    };

    if let Some(pk) = provider_keys {
        for key in &pk.keys {
            // Avoid duplicating the env var key
            if !keys.contains(key) {
                keys.push(key.clone());
            }
        }
    }

    // Only create rotator if we have more than 1 key (rotation is pointless with 1)
    if keys.len() > 1 {
        Some(ApiKeyRotator::new(keys))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_keys_config_parsing() {
        let toml_str = r#"
[gemini]
keys = ["key1", "key2", "key3"]

[anthropic]
keys = ["sk-ant-1", "sk-ant-2"]
"#;
        let config: ApiKeysConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.gemini.as_ref().unwrap().keys.len(), 3);
        assert_eq!(config.anthropic.as_ref().unwrap().keys.len(), 2);
        assert!(config.openai.is_none());
        assert!(config.ollama.is_none());
    }

    #[test]
    fn test_api_key_rotator_round_robin() {
        let rotator = ApiKeyRotator::new(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);

        assert_eq!(rotator.current_key(), "key1");

        // First rotation -> key2
        assert_eq!(rotator.rotate(), Some("key2"));
        assert_eq!(rotator.current_key(), "key2");

        // Second rotation -> key3
        assert_eq!(rotator.rotate(), Some("key3"));
        assert_eq!(rotator.current_key(), "key3");

        // Third rotation -> wraps around, returns None (all exhausted)
        assert_eq!(rotator.rotate(), None);
    }

    #[test]
    fn test_api_key_rotator_single_key() {
        let rotator = ApiKeyRotator::new(vec!["only-key".to_string()]);

        assert_eq!(rotator.current_key(), "only-key");
        assert_eq!(rotator.key_count(), 1);

        // Rotation with single key always returns the same key
        // (wraps around immediately, but len==1 so None since new_idx==0)
        assert_eq!(rotator.rotate(), None);
    }

    #[test]
    fn test_api_key_rotator_reset() {
        let rotator = ApiKeyRotator::new(vec![
            "key1".to_string(),
            "key2".to_string(),
        ]);

        rotator.rotate(); // -> key2
        assert_eq!(rotator.current_key(), "key2");

        rotator.reset();
        assert_eq!(rotator.current_key(), "key1");
    }

    #[test]
    fn test_build_rotator_with_env_and_config() {
        let config = ApiKeysConfig {
            gemini: Some(ProviderKeys {
                keys: vec!["file-key1".to_string(), "file-key2".to_string()],
            }),
            anthropic: None,
            openai: None,
            ollama: None,
        };

        let rotator = build_rotator(
            &config,
            &crate::models::Provider::Gemini,
            "env-key",
        );

        assert!(rotator.is_some());
        let rotator = rotator.unwrap();
        assert_eq!(rotator.key_count(), 3); // env + 2 file keys
        assert_eq!(rotator.current_key(), "env-key");
    }

    #[test]
    fn test_build_rotator_no_rotation_needed() {
        let config = ApiKeysConfig {
            gemini: None,
            anthropic: None,
            openai: None,
            ollama: None,
        };

        // Only 1 key (env var), no rotation needed
        let rotator = build_rotator(
            &config,
            &crate::models::Provider::Gemini,
            "single-key",
        );

        assert!(rotator.is_none());
    }

    #[test]
    fn test_build_rotator_deduplicates_env_key() {
        let config = ApiKeysConfig {
            gemini: Some(ProviderKeys {
                keys: vec!["env-key".to_string(), "other-key".to_string()],
            }),
            anthropic: None,
            openai: None,
            ollama: None,
        };

        let rotator = build_rotator(
            &config,
            &crate::models::Provider::Gemini,
            "env-key", // Same as first file key
        );

        assert!(rotator.is_some());
        let rotator = rotator.unwrap();
        assert_eq!(rotator.key_count(), 2); // Deduplicated: env-key + other-key
    }
}
