//! Cerebras Inference provider configuration and live model discovery.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_BASE_URL: &str = "https://api.cerebras.ai/v1";
pub const PUBLIC_MODELS_URL: &str = "https://api.cerebras.ai/public/v1/models";
pub const KNOWN_PUBLIC_MODELS: &[&str] = &["gemma-4-31b", "gpt-oss-120b"];

pub fn base_url() -> String {
    let configured = env::var("CEREBRAS_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    format!("{}/", configured.trim_end_matches('/'))
}

pub fn key_file_path() -> Result<PathBuf> {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(config_home).join("cerebras.env"));
    }

    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
    Ok(home.join(".config").join("cerebras.env"))
}

fn parse_key_file(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let assignment = line.strip_prefix("export ").unwrap_or(line);
        let (name, value) = assignment.split_once('=')?;
        if name.trim() != "CEREBRAS_API_KEY" {
            return None;
        }

        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        (!value.trim().is_empty()).then(|| value.trim().to_string())
    })
}

fn read_key_file(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("could not read Cerebras key file {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(anyhow!(
                "Cerebras key file permissions are too open ({:04o}); run: chmod 600 {}",
                mode,
                path.display()
            ));
        }
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("could not read Cerebras key file {}", path.display()))?;
    parse_key_file(&contents).ok_or_else(|| {
        anyhow!(
            "Cerebras key file {} does not contain CEREBRAS_API_KEY",
            path.display()
        )
    })
}

/// Resolve credentials from the environment, then the secure config file.
pub fn resolve_key() -> Result<String> {
    if let Ok(key) = env::var("CEREBRAS_API_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }

    let path = key_file_path()?;
    if !path.exists() {
        return Err(anyhow!(
            "CEREBRAS_API_KEY is required; set it or save CEREBRAS_API_KEY=... in {} with file mode 0600",
            path.display()
        ));
    }
    read_key_file(&path)
}

pub fn is_configured() -> bool {
    resolve_key().is_ok()
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<Model>,
}

#[derive(Debug, Deserialize)]
struct Model {
    id: String,
}

/// Fetch the full live Cerebras model catalog. The authenticated endpoint is
/// preferred so the result reflects the caller's account; the public catalog
/// remains available before a key has been configured.
pub fn available_models() -> Result<Vec<String>> {
    let key = resolve_key().ok();
    let url = key
        .as_ref()
        .map(|_| format!("{}models", base_url()))
        .unwrap_or_else(|| PUBLIC_MODELS_URL.to_string());

    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .context("failed to create Cerebras catalog client")?;
        let mut request = client.get(&url);
        if let Some(key) = key {
            request = request.bearer_auth(key);
        }
        let response = request
            .send()
            .context("failed to fetch Cerebras model catalog")?
            .error_for_status()
            .context("Cerebras model catalog returned an error")?;
        let catalog: ModelsResponse = response
            .json()
            .context("failed to parse Cerebras model catalog")?;
        Ok(catalog.data.into_iter().map(|model| model.id).collect())
    })
    .join()
    .map_err(|_| anyhow!("Cerebras model catalog probe panicked"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shell_style_key_files() {
        assert_eq!(
            parse_key_file("# Cerebras\nCEREBRAS_API_KEY=csk_test\n"),
            Some("csk_test".to_string())
        );
        assert_eq!(
            parse_key_file("export CEREBRAS_API_KEY=\"csk_quoted\"\n"),
            Some("csk_quoted".to_string())
        );
        assert_eq!(parse_key_file("OTHER_KEY=nope\n"), None);
    }

    #[test]
    fn reads_secure_key_file() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cerebras.env");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "CEREBRAS_API_KEY=csk_test").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        assert_eq!(read_key_file(&path).unwrap(), "csk_test");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_overly_open_key_file() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cerebras.env");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "CEREBRAS_API_KEY=csk_test").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let error = read_key_file(&path).unwrap_err().to_string();
        assert!(error.contains("permissions are too open"));
    }
}
