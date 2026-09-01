//! Abliteration AI provider configuration.
//!
//! The API is OpenAI-compatible. This module keeps its provider-specific
//! defaults and its `ablit.py`-compatible key lookup in one place.

use anyhow::{anyhow, Result};
use std::env;
use std::path::{Path, PathBuf};

pub const DEFAULT_BASE_URL: &str = "https://api.abliteration.ai/v1";
pub const DEFAULT_MODEL: &str = "abliterated-model-large-v2";
pub const AVAILABLE_MODELS: &[&str] = &[
    DEFAULT_MODEL,
    "abliterated-model-large",
    "abliterated-model",
];
pub const REASONING_EFFORT: &str = "high";
pub const MAX_COMPLETION_TOKENS: u32 = 4096;

pub fn is_model(model: &str) -> bool {
    model.starts_with("abliterated-model")
}

pub fn base_url() -> String {
    let configured = env::var("ABLIT_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    format!("{}/", configured.trim_end_matches('/'))
}

pub fn key_file_path() -> Result<PathBuf> {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(config_home).join("ablit").join("key"));
    }

    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
    Ok(home.join(".config").join("ablit").join("key"))
}

fn read_key_file(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path).map_err(|e| {
        anyhow!(
            "could not read Abliteration key file {}: {}",
            path.display(),
            e
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(anyhow!(
                "Abliteration key file permissions are too open ({:04o}); run: chmod 600 {}",
                mode,
                path.display()
            ));
        }
    }

    let key = std::fs::read_to_string(path).map_err(|e| {
        anyhow!(
            "could not read Abliteration key file {}: {}",
            path.display(),
            e
        )
    })?;
    let key = key.trim();
    if key.is_empty() {
        return Err(anyhow!("Abliteration key file {} is empty", path.display()));
    }
    Ok(key.to_string())
}

/// Resolve credentials in the same order as `ablit.py`: environment first,
/// then the mode-0600 config file.
pub fn resolve_key() -> Result<String> {
    if let Ok(key) = env::var("ABLIT_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }

    let path = key_file_path()?;
    if !path.exists() {
        return Err(anyhow!(
            "ABLIT_KEY required for model '{}'; set ABLIT_KEY or save the key in {} with file mode 0600",
            DEFAULT_MODEL,
            path.display()
        ));
    }
    read_key_file(&path)
}

pub fn is_configured() -> bool {
    env::var("ABLIT_KEY")
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false)
        || key_file_path().map(|path| path.is_file()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_published_model_ids() {
        for model in AVAILABLE_MODELS {
            assert!(is_model(model));
        }
        assert!(!is_model("gpt-5.6-sol"));
    }

    #[test]
    fn reads_secure_key_file() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "ak_test").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        assert_eq!(read_key_file(&path).unwrap(), "ak_test");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_overly_open_key_file() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "ak_test").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let error = read_key_file(&path).unwrap_err().to_string();
        assert!(error.contains("permissions are too open"));
    }
}
