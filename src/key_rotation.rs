//! API Key rotation for handling rate limits and invalid keys
//!
//! Supports two types of rotation:
//! 1. Bad Key (permanent): 403, invalid key errors -> blacklist
//! 2. Quota/Overload (temporary): 429, 503, quota exceeded -> rotate

use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const RETRY_DELAY: Duration = Duration::from_secs(2);
const COOLDOWN_DURATION: Duration = Duration::from_secs(300); // 5 minutes

/// Get the eunice config directory
fn eunice_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".eunice")
}

/// Action to take after a rate limit error
#[derive(Debug, Clone)]
pub enum RateLimitAction {
    /// Wait and retry with same key
    RetryAfterDelay(Duration),
    /// Rotated to a new key, retry immediately
    Rotated { new_key_index: usize },
    /// All keys exhausted, wait for cooldown
    Exhausted { retry_after: Duration },
    /// Cooldown elapsed, reset to first key and retry
    CooldownReset,
}

/// Action to take after a bad key error
#[derive(Debug, Clone)]
pub enum BadKeyAction {
    /// Removed bad key, rotated to next
    Rotated { new_key_index: usize },
    /// All keys are bad
    AllKeysBad,
}

/// Pool of API keys with rotation support
pub struct KeyPool {
    keys: Vec<String>,
    current_index: AtomicUsize,
    retry_attempted: AtomicBool,
    exhausted_at: Mutex<Option<Instant>>,
    blacklist: Mutex<HashSet<String>>,
    key_file: PathBuf,
    index_file: PathBuf,
    blacklist_file: PathBuf,
}

impl KeyPool {
    /// Load Gemini API keys from ~/.eunice/gemini-api-keys.txt
    /// Falls back to GEMINI_API_KEY env var if file doesn't exist
    pub fn load_gemini() -> Result<Self> {
        let eunice_dir = eunice_dir();
        let key_file = eunice_dir.join("gemini-api-keys.txt");
        let index_file = eunice_dir.join("gemini-key-index.txt");
        let blacklist_file = eunice_dir.join("bad-api-keys.txt");

        // Load blacklisted keys
        let blacklist: HashSet<String> = if blacklist_file.exists() {
            fs::read_to_string(&blacklist_file)
                .unwrap_or_default()
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                .map(|s| s.trim().to_string())
                .collect()
        } else {
            HashSet::new()
        };

        // Load keys from file or env
        let keys: Vec<String> = if key_file.exists() {
            fs::read_to_string(&key_file)?
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                .map(|s| s.trim().to_string())
                .filter(|k| !blacklist.contains(k))
                .collect()
        } else if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            if blacklist.contains(&key) {
                return Err(anyhow!("GEMINI_API_KEY is blacklisted as invalid"));
            }
            vec![key]
        } else {
            return Err(anyhow!(
                "No Gemini API keys found. Create ~/.eunice/gemini-api-keys.txt or set GEMINI_API_KEY"
            ));
        };

        if keys.is_empty() {
            return Err(anyhow!("No valid Gemini API keys available (all may be blacklisted)"));
        }

        // Load persisted index
        let saved_index: usize = if index_file.exists() {
            fs::read_to_string(&index_file)
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0)
        } else {
            0
        };

        // Ensure index is in bounds
        let current_index = saved_index % keys.len();

        Ok(Self {
            keys,
            current_index: AtomicUsize::new(current_index),
            retry_attempted: AtomicBool::new(false),
            exhausted_at: Mutex::new(None),
            blacklist: Mutex::new(blacklist),
            key_file,
            index_file,
            blacklist_file,
        })
    }

    /// Create a single-key pool (for non-rotatable providers)
    pub fn single(key: String) -> Self {
        Self {
            keys: vec![key],
            current_index: AtomicUsize::new(0),
            retry_attempted: AtomicBool::new(false),
            exhausted_at: Mutex::new(None),
            blacklist: Mutex::new(HashSet::new()),
            key_file: PathBuf::new(),
            index_file: PathBuf::new(),
            blacklist_file: PathBuf::new(),
        }
    }

    /// Get current active key
    pub fn current_key(&self) -> &str {
        let idx = self.current_index.load(Ordering::SeqCst) % self.keys.len();
        &self.keys[idx]
    }

    /// Get current key index (1-based for display)
    pub fn current_index_display(&self) -> usize {
        (self.current_index.load(Ordering::SeqCst) % self.keys.len()) + 1
    }

    /// Get total number of keys
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Check if this is a single-key pool (no rotation possible)
    pub fn is_single_key(&self) -> bool {
        self.keys.len() == 1
    }

    /// Handle a rate limit error (429, 503, quota exceeded)
    /// Returns the action to take
    pub fn handle_rate_limit(&self) -> RateLimitAction {
        // Check if cooldown has elapsed
        {
            let mut exhausted_at = self.exhausted_at.lock().unwrap();
            if let Some(exhausted_time) = *exhausted_at {
                let elapsed = exhausted_time.elapsed();
                if elapsed >= COOLDOWN_DURATION {
                    // Cooldown elapsed, reset
                    *exhausted_at = None;
                    self.current_index.store(0, Ordering::SeqCst);
                    self.retry_attempted.store(false, Ordering::SeqCst);
                    self.save_index();
                    return RateLimitAction::CooldownReset;
                } else {
                    // Still in cooldown
                    return RateLimitAction::Exhausted {
                        retry_after: COOLDOWN_DURATION - elapsed,
                    };
                }
            }
        }

        // First hit on this key: wait and retry
        if !self.retry_attempted.swap(true, Ordering::SeqCst) {
            return RateLimitAction::RetryAfterDelay(RETRY_DELAY);
        }

        // Second hit: rotate to next key
        self.retry_attempted.store(false, Ordering::SeqCst);

        let current = self.current_index.load(Ordering::SeqCst);
        let next = (current + 1) % self.keys.len();

        // Check if we've cycled through all keys
        if next == 0 && self.keys.len() > 1 {
            // All keys exhausted
            let mut exhausted_at = self.exhausted_at.lock().unwrap();
            *exhausted_at = Some(Instant::now());
            return RateLimitAction::Exhausted {
                retry_after: COOLDOWN_DURATION,
            };
        }

        self.current_index.store(next, Ordering::SeqCst);
        self.save_index();

        RateLimitAction::Rotated { new_key_index: next + 1 }
    }

    /// Handle a bad key error (403, invalid key)
    /// Blacklists the current key and rotates
    pub fn handle_bad_key(&self) -> BadKeyAction {
        let current_key = self.current_key().to_string();

        // Add to blacklist
        {
            let mut blacklist = self.blacklist.lock().unwrap();
            blacklist.insert(current_key.clone());
        }

        // Persist blacklist
        self.save_blacklist();

        // Remove from active keys (can't modify Vec, so just track via blacklist)
        // Find next non-blacklisted key
        let blacklist = self.blacklist.lock().unwrap();
        let valid_keys: Vec<usize> = self.keys
            .iter()
            .enumerate()
            .filter(|(_, k)| !blacklist.contains(*k))
            .map(|(i, _)| i)
            .collect();

        if valid_keys.is_empty() {
            return BadKeyAction::AllKeysBad;
        }

        // Find next valid key after current
        let current = self.current_index.load(Ordering::SeqCst);
        let next = valid_keys
            .iter()
            .find(|&&i| i > current)
            .or_else(|| valid_keys.first())
            .copied()
            .unwrap_or(0);

        self.current_index.store(next, Ordering::SeqCst);
        self.save_index();

        BadKeyAction::Rotated { new_key_index: next + 1 }
    }

    /// Check if the current key is blacklisted (for runtime checks)
    pub fn is_current_key_blacklisted(&self) -> bool {
        let current_key = self.current_key();
        let blacklist = self.blacklist.lock().unwrap();
        blacklist.contains(current_key)
    }

    /// Save current index to file
    fn save_index(&self) {
        if self.index_file.as_os_str().is_empty() {
            return;
        }
        if let Some(parent) = self.index_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&self.index_file, self.current_index.load(Ordering::SeqCst).to_string());
    }

    /// Save blacklist to file
    fn save_blacklist(&self) {
        if self.blacklist_file.as_os_str().is_empty() {
            return;
        }
        if let Some(parent) = self.blacklist_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let blacklist = self.blacklist.lock().unwrap();
        let content: String = blacklist.iter().map(|k| format!("{}\n", k)).collect();
        let _ = fs::write(&self.blacklist_file, content);
    }
}

/// Check if an error indicates a bad/invalid API key
pub fn is_bad_key_error(error_msg: &str) -> bool {
    let lower = error_msg.to_lowercase();

    // HTTP 403
    lower.contains("403")
        || lower.contains("forbidden")
        // Invalid key patterns
        || lower.contains("api key not valid")
        || lower.contains("invalid api key")
        || (lower.contains("api_key") && lower.contains("invalid"))
}

/// Check if an error indicates rate limiting / quota exceeded
pub fn is_quota_error(error_msg: &str) -> bool {
    let lower = error_msg.to_lowercase();

    // HTTP status codes
    (lower.contains("429") || lower.contains("too many requests"))
        || lower.contains("503")
        || lower.contains("service unavailable")
        // Quota patterns
        || lower.contains("quota exceeded")
        || lower.contains("rate limit")
        || (lower.contains("resource_exhausted") && lower.contains("quota"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn test_is_bad_key_error() {
        assert!(is_bad_key_error("403 Forbidden"));
        assert!(is_bad_key_error("API key not valid"));
        assert!(is_bad_key_error("invalid api key"));
        assert!(!is_bad_key_error("429 Too Many Requests"));
        assert!(!is_bad_key_error("quota exceeded"));
    }

    #[test]
    fn test_is_quota_error() {
        assert!(is_quota_error("429 Too Many Requests"));
        assert!(is_quota_error("503 Service Unavailable"));
        assert!(is_quota_error("quota exceeded"));
        assert!(is_quota_error("rate limit exceeded"));
        assert!(!is_quota_error("403 Forbidden"));
        assert!(!is_quota_error("API key not valid"));
    }

    #[test]
    fn test_single_key_pool() {
        let pool = KeyPool::single("test-key".to_string());
        assert_eq!(pool.current_key(), "test-key");
        assert!(pool.is_single_key());
        assert_eq!(pool.key_count(), 1);
    }

    #[test]
    fn test_rate_limit_retry_then_rotate() {
        let pool = KeyPool {
            keys: vec!["key1".to_string(), "key2".to_string(), "key3".to_string()],
            current_index: AtomicUsize::new(0),
            retry_attempted: AtomicBool::new(false),
            exhausted_at: Mutex::new(None),
            blacklist: Mutex::new(HashSet::new()),
            key_file: PathBuf::new(),
            index_file: PathBuf::new(),
            blacklist_file: PathBuf::new(),
        };

        // First call: should retry with delay
        match pool.handle_rate_limit() {
            RateLimitAction::RetryAfterDelay(d) => assert_eq!(d, RETRY_DELAY),
            _ => panic!("Expected RetryAfterDelay"),
        }

        // Second call: should rotate
        match pool.handle_rate_limit() {
            RateLimitAction::Rotated { new_key_index } => assert_eq!(new_key_index, 2),
            _ => panic!("Expected Rotated"),
        }

        assert_eq!(pool.current_key(), "key2");
    }

    #[test]
    fn test_bad_key_blacklist() {
        let pool = KeyPool {
            keys: vec!["key1".to_string(), "key2".to_string()],
            current_index: AtomicUsize::new(0),
            retry_attempted: AtomicBool::new(false),
            exhausted_at: Mutex::new(None),
            blacklist: Mutex::new(HashSet::new()),
            key_file: PathBuf::new(),
            index_file: PathBuf::new(),
            blacklist_file: PathBuf::new(),
        };

        match pool.handle_bad_key() {
            BadKeyAction::Rotated { new_key_index } => assert_eq!(new_key_index, 2),
            _ => panic!("Expected Rotated"),
        }

        assert_eq!(pool.current_key(), "key2");

        // key1 should be blacklisted
        let blacklist = pool.blacklist.lock().unwrap();
        assert!(blacklist.contains("key1"));
    }
}
