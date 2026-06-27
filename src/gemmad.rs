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
            if h == "0.0.0.0" {
                "127.0.0.1".to_string()
            } else {
                h.to_string()
            }
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

/// Fallback model id (env override, else the daemon's current default). The
/// authoritative id comes from `live_model_id()`; this is only used when that
/// query fails.
pub fn model_id() -> String {
    std::env::var("GEMMAD_MODEL_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gemma-4-26b-a4b".to_string())
}

pub fn base_url() -> String {
    format!("http://{}:{}/v1/", host(), port())
}

/// Query the daemon's `/v1/models` for the live model id. The `model` field a
/// client sends is advisory — the server always uses whichever model is loaded
/// — so this reports the real id for display and the request body. Returns
/// `None` on any failure (caller falls back to `model_id()`).
pub async fn live_model_id() -> Option<String> {
    let token = resolve_token().ok()?;
    let url = format!("http://{}:{}/v1/models", host(), port());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
        .ok()?;
    let resp = client.get(&url).bearer_auth(token).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("data")?
        .as_array()?
        .first()?
        .get("id")?
        .as_str()
        .map(|s| s.to_string())
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
