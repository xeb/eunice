# API Key Rotation Feature

## Overview

Automatically rotate API keys when hitting rate limits or quota errors. Starting with Gemini only.

## Key File Location

```
~/.eunice/gemini-api-keys.txt
```

Format: One API key per line, empty lines and `#` comments ignored.

```
# Production keys
AIzaSyA...key1
AIzaSyB...key2
AIzaSyC...key3
```

## Current Architecture

```
provider.rs::detect_provider()
    -> reads GEMINI_API_KEY from env
    -> returns ProviderInfo { api_key: String, ... }

client.rs::Client
    -> stores api_key: String
    -> uses it in add_auth() and native Gemini requests
```

## Proposed Architecture

### 1. New Module: `src/key_rotation.rs`

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::sync::Mutex;

const RETRY_DELAY: Duration = Duration::from_secs(2);
const COOLDOWN_DURATION: Duration = Duration::from_secs(300); // 5 minutes

pub struct KeyPool {
    keys: Vec<String>,
    current_index: AtomicUsize,
    exhausted_at: Mutex<Option<Instant>>,  // When all keys were exhausted
    state_file: PathBuf,  // ~/.eunice/gemini-key-index.txt
}

impl KeyPool {
    /// Load keys from file, fall back to env var if file doesn't exist
    /// Also loads persisted key index
    pub fn load_gemini() -> Result<Self>;

    /// Get current active key
    pub fn current_key(&self) -> &str;

    /// Handle rate limit: returns action to take
    pub fn handle_rate_limit(&self) -> RateLimitAction;

    /// Rotate to next key, persists index
    fn rotate(&self) -> bool;

    /// Check if cooldown has elapsed and reset if so
    fn check_cooldown(&self) -> bool;

    /// Persist current index to file
    fn save_index(&self);
}

pub enum RateLimitAction {
    /// Wait 2s and retry with same key
    RetryAfterDelay,
    /// Rotated to a new key, retry immediately
    Rotated,
    /// All keys exhausted, wait for cooldown
    Exhausted { retry_after: Duration },
    /// Cooldown elapsed, retrying from first key
    CooldownReset,
}
```
```

### 2. Modify Client

```rust
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    key_pool: Arc<KeyPool>,  // Changed from api_key: String
    provider: Provider,
    // ...
}

impl Client {
    /// Rotate to next API key (called on rate limit errors)
    pub fn rotate_key(&self) -> bool;
}
```

### 3. Error Detection & Rotation Types

**Bad Key Rotation (Permanent Removal)**

Key is invalid - remove from pool and blacklist in `~/.eunice/bad-api-keys.txt`:
- HTTP 403 (Forbidden)
- Error contains "api key not valid"
- Error contains "api_key" (invalid key error patterns)

**Quota/Overload Rotation (Temporary Skip)**

Rate limited - rotate to next key but keep current key in pool:
- HTTP 429 (Too Many Requests)
- HTTP 503 (Service Unavailable)
- Error contains "quota exceeded"
- Error contains "RESOURCE_EXHAUSTED" (when quota-related)

Retry logic:
1. First hit: wait 2s, retry same key
2. Second hit: rotate to next key, retry immediately
3. All keys exhausted: wait 5min cooldown, then reset to first key

### 4. Integration Points

**agent.rs** - Main agent loop error handling:
```rust
Err(e) => {
    let error_msg = format!("{:#}", e);

    // Try key rotation on rate limit
    if is_rate_limit_error(&error_msg) && client.rotate_key() {
        display.write_event(DisplayEvent::Info {
            message: "Rate limited, rotating API key...".into(),
        });
        continue; // Retry with new key
    }

    // Existing compaction logic...
}
```

**webapp/handlers.rs** - Same pattern for webapp agent loop.

## File Paths by Provider

| Provider | Key File |
|----------|----------|
| Gemini | `~/.eunice/gemini-api-keys.txt` |
| OpenAI | `~/.eunice/openai-api-keys.txt` (future) |
| Anthropic | `~/.eunice/anthropic-api-keys.txt` (future) |

## Fallback Behavior

1. If key file exists and has keys -> use key pool
2. If key file doesn't exist -> fall back to env var (single key)
3. If neither exists -> error as before

## Display Feedback

When rotating:
```
[Rate limited on key #1, trying key #2...]
```

When exhausted:
```
[All API keys exhausted, waiting for rate limit reset...]
```

## Design Decisions

1. **Persist key index:** Yes - save last-used key index to `~/.eunice/gemini-key-index.txt`
2. **Track per-key failures:** No - keep it simple
3. **Retry behavior:** 1 retry after 2 second delay, then rotate to next key
4. **Exhausted key cooldown:** Yes - 5 minute cooldown, then retry exhausted keys

## Implementation Order

1. [x] Create `src/key_rotation.rs` with `KeyPool` struct
2. [x] Modify `Client` to use `KeyPool` instead of single key
3. [x] Update `detect_provider()` to load key pool (via Client::new)
4. [x] Add rotation logic to agent.rs error handling
5. [x] Add rotation logic to webapp/handlers.rs
6. [x] Add display messages for rotation events (Info event type)
7. [x] Add tests (4 unit tests in key_rotation.rs)
8. [ ] Update CLAUDE.md with key file documentation

## Files Modified

- `src/key_rotation.rs` - New module with KeyPool, error detection
- `src/client.rs` - Uses KeyPool, exposes rotation methods
- `src/agent.rs` - Handles bad key and quota errors with rotation
- `src/webapp/handlers.rs` - Same rotation logic for webapp
- `src/display_sink.rs` - Added Info event type
- `src/lib.rs`, `src/main.rs` - Module declarations
