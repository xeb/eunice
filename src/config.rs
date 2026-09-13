//! Optional per-user defaults. Explicit model choices never require this file.
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    default_model: Option<String>,
}

pub fn default_model() -> Result<Option<String>> {
    let home = dirs::home_dir().context("Cannot locate home directory for ~/.eunice/config.toml")?;
    load_default_model(&home.join(".eunice/config.toml"))
}

fn load_default_model(path: &Path) -> Result<Option<String>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("Cannot read {}", path.display())),
    };
    let config: Config = toml::from_str(&text)
        .with_context(|| format!("Invalid configuration in {}", path.display()))?;
    match config.default_model {
        Some(model) if model.trim().is_empty() => bail!("default_model must not be empty in {}", path.display()),
        Some(model) => Ok(Some(model.trim().to_owned())),
        None => Ok(None),
    }
}

/// Defer reading configuration until the caller actually needs a default.
fn with_default<F>(explicit: Option<&str>, load: F) -> Result<Option<String>>
where F: FnOnce() -> Result<Option<String>> {
    match explicit {
        Some(model) => Ok(Some(model.to_owned())),
        None => load(),
    }
}

pub fn model_or_default(explicit: Option<&str>) -> Result<Option<String>> {
    with_default(explicit, default_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_empty_config_preserves_automatic_selection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert_eq!(load_default_model(&path).unwrap(), None);
        std::fs::write(&path, "# optional defaults\n").unwrap();
        assert_eq!(load_default_model(&path).unwrap(), None);
    }

    #[test]
    fn reads_and_trims_local_or_cloud_model_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for model in ["hf:qwen3.5:2b", "sonnet", "astra"] {
            std::fs::write(&path, format!("default_model = ' {model} '\n")).unwrap();
            assert_eq!(load_default_model(&path).unwrap().as_deref(), Some(model));
        }
    }

    #[test]
    fn malformed_defaults_do_not_silently_fall_back_to_another_provider() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for text in ["default_model =", "default_model = 42", "default_model = '  '", "default_modle = 'sonnet'"] {
            std::fs::write(&path, text).unwrap();
            let error = load_default_model(&path).unwrap_err().to_string();
            assert!(error.contains(path.to_str().unwrap()), "{error}");
        }
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(load_default_model(&path).unwrap_err().to_string().contains("Cannot read"));
    }

    #[test]
    fn explicit_model_overrides_even_broken_configuration() {
        assert_eq!(with_default(Some("sonnet"), || bail!("must not read config")).unwrap().as_deref(), Some("sonnet"));
        assert_eq!(with_default(None, || Ok(Some("hf:qwen3.5:2b".into()))).unwrap().as_deref(), Some("hf:qwen3.5:2b"));
        assert!(with_default(None, || bail!("invalid config")).is_err());
    }
}
