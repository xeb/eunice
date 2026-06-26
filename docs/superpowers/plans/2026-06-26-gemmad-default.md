# `--gemmad` Daemon Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect an already-running `gemmad` daemon (local gemma-4-12b, OpenAI-compatible, on `:18082`) and route to it — as the global default for plain `eunice`, forceable via `--gemmad`, opt-out via `--no-gemmad`.

**Architecture:** A new `src/gemmad.rs` module owns daemon constants, `/livez` detection, Bearer-token resolution, and a pure `decide_model()` precedence function. A new `Provider::Gemmad` variant routes through the existing OpenAI-compatible client path with Bearer auth. The 31B `--gemma` MTP build path is untouched.

**Tech Stack:** Rust, reqwest (rustls), clap, anyhow, serde. No new crates (keys.toml is hand-parsed).

## Global Constraints

- Daemon defaults: host `127.0.0.1`, port `18082`, model id `gemma-4-12b`. Overridable via `GEMMAD_HOST` / `GEMMAD_PORT` / `GEMMAD_MODEL_ID` (the same env vars gemmad uses). A `GEMMAD_HOST` of `0.0.0.0` is mapped to `127.0.0.1` for the client connection.
- Token precedence: `$GEMMAD_API_KEY` → `~/.config/gemmad/keys.toml` `[keys].dev` (else first key) → error. Keys-file path overridable via `GEMMAD_KEYS_FILE`.
- **Runtime fact (verified against the live daemon):** gemmad returns HTTP 200 for requests with `tools`, but does **not** emit OpenAI `tool_calls` — it inlines Gemma tool-call tokens (`<|tool_call>…`) into `message.content`. eunice therefore must **withhold tools** when talking to gemmad. `supports_tools()` only drives a stderr warning; the real enforcement is a `Client.send_tools` flag.
- `--gemma` (31B MTP build) behavior is unchanged.
- Mutually-exclusive: `--gemma`+`--gemmad`, `--gemmad`+`--no-gemmad`, `--gemmad`+`--model X` (X≠gemmad model id).

---

### Task 1: Add `Provider::Gemmad` variant + satisfy all exhaustive matches

**Files:**
- Modify: `src/models.rs` (enum, `Display`, `get_icon`)
- Modify: `src/provider.rs` (`supports_tools`)
- Modify: `src/usage.rs` (`get_pricing`)

**Interfaces:**
- Produces: `models::Provider::Gemmad` used by every later task.

- [ ] **Step 1: Add the enum variant + Display + icon**

In `src/models.rs`, add `Gemmad` to the enum (after `Local`):
```rust
pub enum Provider {
    OpenAI,
    Gemini,
    Anthropic,
    Ollama,
    AzureOpenAI,
    Local,
    Gemmad,
}
```
Add to the `Display` match:
```rust
            Provider::Gemmad => write!(f, "Gemmad"),
```
Add to `get_icon` match:
```rust
            Provider::Gemmad => "💻",
```

- [ ] **Step 2: Add `supports_tools` arm (false) with rationale**

In `src/provider.rs::supports_tools`, add before the `Provider::Ollama` arm:
```rust
        // gemmad (local gemma-4-12b) speaks OpenAI chat but does NOT emit
        // OpenAI tool_calls — it inlines Gemma tool-call tokens into content.
        // Treat as text-only so the warning fires and Client withholds tools.
        Provider::Gemmad => false,
```

- [ ] **Step 3: Add pricing arm (free/local)**

In `src/usage.rs::get_pricing`, add after the `Provider::Local` arm:
```rust
        Provider::Gemmad => {
            // Local daemon inference is free
            (0.0, 0.0)
        }
```

- [ ] **Step 4: Write the test (provider.rs tests module)**

Add to `src/provider.rs` `mod tests`:
```rust
    #[test]
    fn test_gemmad_is_text_only() {
        assert!(!supports_tools(&Provider::Gemmad, "gemma-4-12b"));
    }
```

- [ ] **Step 5: Verify it compiles and passes**

Run: `cargo test --lib test_gemmad_is_text_only`
Expected: PASS, and the crate compiles (all exhaustive matches satisfied).

- [ ] **Step 6: Commit**

```bash
git add src/models.rs src/provider.rs src/usage.rs
git commit -m "Add Provider::Gemmad variant (text-only, free)"
```

---

### Task 2: New `src/gemmad.rs` module (config, detection, token, decide_model)

**Files:**
- Create: `src/gemmad.rs`
- Modify: `src/main.rs` (add `mod gemmad;`)

**Interfaces:**
- Produces:
  - `gemmad::host() -> String`, `gemmad::port() -> u16`, `gemmad::model_id() -> String`, `gemmad::base_url() -> String`
  - `async gemmad::is_available() -> bool`
  - `gemmad::resolve_token() -> anyhow::Result<String>`
  - `enum gemmad::ModelChoice { Explicit(String), Gemmad, Gemma31b, SmartDefault }`
  - `gemmad::decide_model(gemma: bool, gemmad: bool, no_gemmad: bool, model: Option<&str>, gemmad_up: bool) -> Result<ModelChoice>`

- [ ] **Step 1: Write the module with tests**

Create `src/gemmad.rs`:
```rust
//! gemmad daemon integration.
//!
//! `gemmad` is a separate, already-running OpenAI-compatible server (local
//! gemma-4-12b on :18082, Bearer-auth). When it is reachable it becomes the
//! global default model for eunice. This module owns detection, token
//! resolution, and the model-selection precedence.

use anyhow::{anyhow, bail, Result};
use std::path::PathBuf;
use std::time::Duration;

/// Client-side host for the daemon. gemmad binds 0.0.0.0; we connect to
/// loopback, so a configured `0.0.0.0` is mapped to `127.0.0.1`.
pub fn host() -> String {
    match std::env::var("GEMMAD_HOST") {
        Ok(h) if !h.trim().is_empty() => {
            let h = h.trim();
            if h == "0.0.0.0" { "127.0.0.1".to_string() } else { h.to_string() }
        }
        _ => "127.0.0.1".to_string(),
    }
}

pub fn port() -> u16 {
    std::env::var("GEMMAD_PORT")
        .ok()
        .and_then(|p| p.trim().parse().ok())
        .unwrap_or(18082)
}

pub fn model_id() -> String {
    std::env::var("GEMMAD_MODEL_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gemma-4-12b".to_string())
}

pub fn base_url() -> String {
    format!("http://{}:{}/v1/", host(), port())
}

/// Probe the daemon's unauthenticated `/livez` endpoint with a short timeout.
pub async fn is_available() -> bool {
    let url = format!("http://{}:{}/livez", host(), port());
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(400))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client.get(&url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

fn keys_path() -> PathBuf {
    if let Ok(p) = std::env::var("GEMMAD_KEYS_FILE") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".config").join("gemmad").join("keys.toml")
}

/// Minimal parser for gemmad's keys.toml: a `[keys]` table of `label = "token"`.
/// Returns the `dev` token if present, else the first token found.
fn parse_keys_toml(content: &str) -> Option<String> {
    let mut in_keys = false;
    let mut first: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_keys = line == "[keys]";
            continue;
        }
        if !in_keys {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"').trim().to_string();
            if v.is_empty() {
                continue;
            }
            if k == "dev" {
                return Some(v);
            }
            if first.is_none() {
                first = Some(v);
            }
        }
    }
    first
}

/// Resolve the Bearer token: env override, then keys.toml, else error.
pub fn resolve_token() -> Result<String> {
    if let Ok(tok) = std::env::var("GEMMAD_API_KEY") {
        if !tok.trim().is_empty() {
            return Ok(tok.trim().to_string());
        }
    }
    let path = keys_path();
    let content = std::fs::read_to_string(&path).map_err(|e| {
        anyhow!(
            "gemmad token not found: set GEMMAD_API_KEY or add a key to {} ({})",
            path.display(),
            e
        )
    })?;
    parse_keys_toml(&content).ok_or_else(|| {
        anyhow!(
            "gemmad token not found: no [keys] entry in {} — set GEMMAD_API_KEY or add one",
            path.display()
        )
    })
}

/// The resolved model-selection decision (network-free; `gemmad_up` is injected).
#[derive(Debug, PartialEq)]
pub enum ModelChoice {
    Explicit(String),
    Gemmad,
    Gemma31b,
    SmartDefault,
}

/// Decide which model to use from the flags and whether the daemon is up.
/// Pure: no network, no env beyond `model_id()` for the conflict check.
pub fn decide_model(
    gemma: bool,
    gemmad: bool,
    no_gemmad: bool,
    model: Option<&str>,
    gemmad_up: bool,
) -> Result<ModelChoice> {
    if gemma && gemmad {
        bail!("--gemma and --gemmad cannot be used together");
    }
    if gemmad && no_gemmad {
        bail!("--gemmad and --no-gemmad cannot be used together");
    }
    if gemma {
        if let Some(m) = model {
            if m != "gemma4:31b" {
                bail!(
                    "--gemma is shorthand for --model=gemma4:31b and cannot be combined with --model={}",
                    m
                );
            }
        }
        return Ok(ModelChoice::Gemma31b);
    }
    if gemmad {
        if let Some(m) = model {
            if m != model_id() {
                bail!(
                    "--gemmad uses the running gemmad daemon and cannot be combined with --model={}",
                    m
                );
            }
        }
        if !gemmad_up {
            bail!(
                "--gemmad requested but no gemmad daemon is reachable at {}:{} — \
                 start it (e.g. `systemctl --user start gemmad`) or omit --gemmad",
                host(),
                port()
            );
        }
        return Ok(ModelChoice::Gemmad);
    }
    if let Some(m) = model {
        return Ok(ModelChoice::Explicit(m.to_string()));
    }
    if !no_gemmad && gemmad_up {
        return Ok(ModelChoice::Gemmad);
    }
    Ok(ModelChoice::SmartDefault)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keys_prefers_dev() {
        let toml = "# comment\n[keys]\nprogeny = \"sk-progeny\"\ndev = \"sk-dev\"\n";
        assert_eq!(parse_keys_toml(toml), Some("sk-dev".to_string()));
    }

    #[test]
    fn test_parse_keys_falls_back_to_first() {
        let toml = "[keys]\nwatcher = \"sk-watch\"\nother = \"sk-other\"\n";
        assert_eq!(parse_keys_toml(toml), Some("sk-watch".to_string()));
    }

    #[test]
    fn test_parse_keys_empty() {
        assert_eq!(parse_keys_toml("[keys]\n# nothing\n"), None);
        assert_eq!(parse_keys_toml("[other]\nx = \"y\"\n"), None);
    }

    #[test]
    fn test_decide_gemma_unchanged() {
        assert_eq!(
            decide_model(true, false, false, None, false).unwrap(),
            ModelChoice::Gemma31b
        );
        assert_eq!(
            decide_model(true, false, false, Some("gemma4:31b"), false).unwrap(),
            ModelChoice::Gemma31b
        );
        assert!(decide_model(true, false, false, Some("gpt-5"), false).is_err());
    }

    #[test]
    fn test_decide_gemmad_forced() {
        assert_eq!(
            decide_model(false, true, false, None, true).unwrap(),
            ModelChoice::Gemmad
        );
        // forced but daemon down -> error
        assert!(decide_model(false, true, false, None, false).is_err());
        // forced + conflicting model -> error
        assert!(decide_model(false, true, false, Some("gpt-5"), true).is_err());
    }

    #[test]
    fn test_decide_global_default() {
        // nothing specified, daemon up -> gemmad
        assert_eq!(
            decide_model(false, false, false, None, true).unwrap(),
            ModelChoice::Gemmad
        );
        // nothing specified, daemon down -> smart default
        assert_eq!(
            decide_model(false, false, false, None, false).unwrap(),
            ModelChoice::SmartDefault
        );
        // opt out even when up -> smart default
        assert_eq!(
            decide_model(false, false, true, None, true).unwrap(),
            ModelChoice::SmartDefault
        );
        // explicit model wins over daemon
        assert_eq!(
            decide_model(false, false, false, Some("sonnet"), true).unwrap(),
            ModelChoice::Explicit("sonnet".to_string())
        );
    }

    #[test]
    fn test_decide_mutually_exclusive() {
        assert!(decide_model(true, true, false, None, true).is_err());
        assert!(decide_model(false, true, true, None, true).is_err());
    }
}
```

- [ ] **Step 2: Register the module**

In `src/main.rs`, add alongside the other `mod` lines (e.g. after `mod display_sink;`):
```rust
mod gemmad;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib gemmad::`
Expected: all `gemmad` tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/gemmad.rs src/main.rs
git commit -m "Add src/gemmad.rs: detection, token, decide_model"
```

---

### Task 3: Route `Provider::Gemmad` in `detect_provider`

**Files:**
- Modify: `src/provider.rs::detect_provider`
- Test: `src/provider.rs` tests module

**Interfaces:**
- Consumes: `gemmad::model_id()`, `gemmad::base_url()`, `gemmad::resolve_token()`.
- Produces: `detect_provider(gemmad_model_id)` → `ProviderInfo { provider: Gemmad, … }`.

- [ ] **Step 1: Add the early routing branch**

In `src/provider.rs::detect_provider`, insert immediately after the `ollama_host` line (before the `// 1. Check for Gemini` block):
```rust
    // 0. gemmad daemon — the local OpenAI-compatible server (gemma-4-12b on :18082).
    // Bearer-auth; no local server is started (handled by the running daemon).
    if model == crate::gemmad::model_id() {
        return Ok(ProviderInfo {
            provider: Provider::Gemmad,
            base_url: crate::gemmad::base_url(),
            api_key: crate::gemmad::resolve_token()?,
            resolved_model: format!("{} (local gemmad)", crate::gemmad::model_id()),
            use_native_gemini_api: false,
            azure_api_version: None,
        });
    }
```

- [ ] **Step 2: Write the test**

Add to `src/provider.rs` `mod tests`:
```rust
    #[test]
    fn test_gemmad_model_routes_to_gemmad_provider() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("GEMMAD_API_KEY", "sk-test-token");

        let info = detect_provider("gemma-4-12b").expect("gemmad model resolves");
        assert_eq!(info.provider, Provider::Gemmad);
        assert_eq!(info.base_url, "http://127.0.0.1:18082/v1/");
        assert_eq!(info.api_key, "sk-test-token");
        assert!(info.resolved_model.contains("gemma-4-12b"));
        assert!(!info.use_native_gemini_api);

        std::env::remove_var("GEMMAD_API_KEY");
    }
```

- [ ] **Step 3: Run the test**

Run: `cargo test --lib test_gemmad_model_routes_to_gemmad_provider`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/provider.rs
git commit -m "Route gemmad model id to Provider::Gemmad with Bearer auth"
```

---

### Task 4: Bearer auth + withhold tools in the `Client`

**Files:**
- Modify: `src/client.rs` (struct field, `with_key_pool`, `add_auth`, `chat_completion`)
- Test: `src/client.rs` tests module

**Interfaces:**
- Consumes: `Provider::Gemmad`.
- Produces: a `Client` that sends `Authorization: Bearer <token>` and omits `tools` for gemmad.

- [ ] **Step 1: Add the `send_tools` field**

In `src/client.rs`, add to the `Client` struct (after `debug: bool,`):
```rust
    /// Whether to forward tool definitions to the provider. gemmad does not
    /// emit OpenAI tool_calls, so tools are withheld for it.
    send_tools: bool,
```

- [ ] **Step 2: Set it in `with_key_pool`**

In the `Ok(Self { … })` constructor in `with_key_pool`, add:
```rust
            send_tools: !matches!(provider_info.provider, Provider::Gemmad),
```

- [ ] **Step 3: Add an explicit Bearer arm in `add_auth`**

In `add_auth`, add before the `_ =>` arm:
```rust
            Provider::Gemmad => req.header(AUTHORIZATION, format!("Bearer {}", api_key)),
```

- [ ] **Step 4: Withhold tools in `chat_completion`**

In `chat_completion`, immediately before `let request = ChatCompletionRequest {`:
```rust
        // gemmad (and any text-only provider) gets no tool definitions.
        let tools = if self.send_tools { tools } else { None };
```
(The existing `tools.map(...)` lines for `tools`/`tool_choice` now see the shadowed value. `chat_completion_streaming` for non-Gemini providers already delegates to `chat_completion`, so this single chokepoint covers single-shot, TUI, and webapp.)

- [ ] **Step 5: Write the test**

Append to `src/client.rs` (add a `#[cfg(test)] mod tests { … }` if none exists, else add inside the existing one):
```rust
#[cfg(test)]
mod gemmad_client_tests {
    use super::*;
    use crate::models::{Provider, ProviderInfo};

    fn gemmad_info() -> ProviderInfo {
        ProviderInfo {
            provider: Provider::Gemmad,
            base_url: "http://127.0.0.1:18082/v1/".to_string(),
            api_key: "sk-test".to_string(),
            resolved_model: "gemma-4-12b".to_string(),
            use_native_gemini_api: false,
            azure_api_version: None,
        }
    }

    #[test]
    fn test_gemmad_client_withholds_tools() {
        let client = Client::new(&gemmad_info()).unwrap();
        assert!(!client.send_tools);
    }

    #[test]
    fn test_gemmad_client_sends_bearer() {
        let client = Client::new(&gemmad_info()).unwrap();
        let req = client
            .add_auth(client.http.post("http://127.0.0.1:18082/v1/chat/completions"))
            .build()
            .unwrap();
        let auth = req
            .headers()
            .get(AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(auth, "Bearer sk-test");
    }
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test --lib gemmad_client_tests`
Expected: both PASS.

- [ ] **Step 7: Commit**

```bash
git add src/client.rs
git commit -m "Client: Bearer auth for gemmad and withhold tools (no OpenAI tool_calls)"
```

---

### Task 5: Wire `--gemmad` / `--no-gemmad` flags + model selection in `main.rs`

**Files:**
- Modify: `src/main.rs` (Args struct, model-selection block, arg tests)

**Interfaces:**
- Consumes: `gemmad::{is_available, decide_model, model_id, host, port, ModelChoice}`.

- [ ] **Step 1: Add the flags**

In `src/main.rs` `struct Args`, after the `gemma` field:
```rust
    /// Use the already-running gemmad daemon (local gemma-4-12b); error if unreachable
    #[arg(long)]
    gemmad: bool,

    /// Do not auto-use a running gemmad daemon; fall back to the cloud smart-default
    #[arg(long)]
    no_gemmad: bool,
```

- [ ] **Step 2: Replace the model-selection block**

Replace the existing block (the `let model = if args.gemma { … } else { … };` at `src/main.rs:405-420`) with:
```rust
    // Select model. A running gemmad daemon is the global default; --gemmad
    // forces it, --no-gemmad opts out, --gemma still builds the 31B MTP server.
    let need_probe =
        args.gemmad || (!args.gemma && args.model.is_none() && !args.no_gemmad);
    let gemmad_up = if need_probe { gemmad::is_available().await } else { false };
    let choice = gemmad::decide_model(
        args.gemma,
        args.gemmad,
        args.no_gemmad,
        args.model.as_deref(),
        gemmad_up,
    )?;
    let used_gemmad = matches!(choice, gemmad::ModelChoice::Gemmad);
    let model = match choice {
        gemmad::ModelChoice::Explicit(m) => m,
        gemmad::ModelChoice::Gemmad => gemmad::model_id(),
        gemmad::ModelChoice::Gemma31b => "gemma4:31b".to_string(),
        gemmad::ModelChoice::SmartDefault => get_smart_default_model()?,
    };
    if used_gemmad {
        eprintln!(
            "Using local gemmad ({}) at {}:{}",
            gemmad::model_id(),
            gemmad::host(),
            gemmad::port()
        );
    }
```

- [ ] **Step 3: Add arg-parsing tests**

Add to `src/main.rs` `mod tests`:
```rust
    #[test]
    fn test_args_gemmad_flag() {
        let args = Args::try_parse_from(["eunice", "--gemmad", "hi"]).unwrap();
        assert!(args.gemmad);
        assert!(!args.no_gemmad);
    }

    #[test]
    fn test_args_no_gemmad_flag() {
        let args = Args::try_parse_from(["eunice", "--no-gemmad", "hi"]).unwrap();
        assert!(args.no_gemmad);
        assert!(!args.gemmad);
    }

    #[test]
    fn test_args_gemmad_default_false() {
        let args = Args::try_parse_from(["eunice", "hi"]).unwrap();
        assert!(!args.gemmad);
        assert!(!args.no_gemmad);
    }
```

- [ ] **Step 4: Run tests + full build**

Run: `cargo test --lib`
Expected: all tests PASS (102 existing + new), crate builds.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "Wire --gemmad/--no-gemmad flags and gemmad-default model selection"
```

---

### Task 6: Docs, full test, and live verification

**Files:**
- Modify: `README.md` (document flags; update LOC per CLAUDE.md)

- [ ] **Step 1: Document the flags in README**

Add `--gemmad` / `--no-gemmad` to the relevant flags/usage section of `README.md`, e.g.:
```
--gemmad      Use the already-running local gemmad daemon (gemma-4-12b on :18082).
              A reachable daemon is the default when no model is specified; errors
              if --gemmad is given but the daemon is not reachable. Token comes from
              $GEMMAD_API_KEY or ~/.config/gemmad/keys.toml. Tools are disabled
              (the daemon does not emit OpenAI tool_calls).
--no-gemmad   Ignore a running gemmad daemon; use the cloud smart-default instead.
```

- [ ] **Step 2: Update LOC count in README**

Run the LOC command from `CLAUDE.md`:
```bash
total=0
for file in src/*.rs src/tools/*.rs src/webapp/*.rs src/tui/*.rs; do
  test -f "$file" || continue
  test_start=$(grep -n "^#\[cfg(test)\]" "$file" 2>/dev/null | cut -d: -f1 | head -1)
  if [ -n "$test_start" ]; then lines=$((test_start - 1)); else lines=$(wc -l < "$file"); fi
  total=$((total + lines))
done
echo "Total: $total lines"
```
Update the LOC figure in `README.md` to match.

- [ ] **Step 3: Full test run**

Run: `cargo test`
Expected: all tests PASS.

- [ ] **Step 4: Live verification against the running daemon**

Confirm gemmad is up, then run a real single-shot and a chat smoke test:
```bash
systemctl --user is-active gemmad        # expect: active
cargo run --release -- "In one sentence, what is the capital of France?"
# Expect stderr: "Using local gemmad (gemma-4-12b) at 127.0.0.1:18082"
# Expect a clean text answer (no <|tool_call> artifacts).
cargo run --release -- --no-gemmad "ping"   # expect: routes to cloud smart-default
```
Also verify the failing original command now succeeds via the daemon:
```bash
cargo run --release -- --gemmad --chat   # interactive; Ctrl-D/escape to exit
```

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "Document --gemmad/--no-gemmad; update LOC"
```

---

## Self-Review

- **Spec coverage:** Global-default semantics (Task 5), `--gemmad`/`--no-gemmad` (Task 5), `/livez` detection (Task 2), `Provider::Gemmad` + Bearer (Tasks 1,3,4), token precedence with env override (Task 2), tools-withholding for the verified no-tool_calls behavior (Task 4), `--gemma` unchanged (Task 2 `decide_model`), tests (every task), docs/LOC (Task 6). All covered.
- **Placeholder scan:** none — every step has concrete code/commands.
- **Type consistency:** `ModelChoice` variants and `decide_model` signature are identical in Task 2 (definition) and Task 5 (consumption); `send_tools` field name consistent across Task 4 steps; `Provider::Gemmad` consistent everywhere.
