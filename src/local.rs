use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 18921;
const SERVER_BINARY_NAME: &str = "gemma4-server";
const HEALTH_TIMEOUT_SECS: u64 = 120;
const HEALTH_POLL_INTERVAL_MS: u64 = 500;

/// Resolved HuggingFace model info
pub struct HfModelInfo {
    pub repo: String,
    pub filename: String,
    pub display_name: String,
    pub size_hint: &'static str,
}

/// Resolve a model alias like "gemma4:e4b" to HuggingFace coordinates
pub fn resolve_hf_alias(alias: &str) -> HfModelInfo {
    match alias {
        "gemma4:e4b" | "gemma4:e4b-q4" => HfModelInfo {
            repo: "unsloth/gemma-4-E4B-it-GGUF".to_string(),
            filename: "gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
            display_name: "gemma-4-E4B-it-Q4_K_M".to_string(),
            size_hint: "~4.5 GB",
        },
        "gemma4:e4b-q8" => HfModelInfo {
            repo: "unsloth/gemma-4-E4B-it-GGUF".to_string(),
            filename: "gemma-4-E4B-it-Q8_0.gguf".to_string(),
            display_name: "gemma-4-E4B-it-Q8_0".to_string(),
            size_hint: "~8 GB",
        },
        "gemma4:e4b-q5" => HfModelInfo {
            repo: "unsloth/gemma-4-E4B-it-GGUF".to_string(),
            filename: "gemma-4-E4B-it-Q5_K_M.gguf".to_string(),
            display_name: "gemma-4-E4B-it-Q5_K_M".to_string(),
            size_hint: "~5.5 GB",
        },
        "gemma4:26b" | "gemma4:26b-q4" => HfModelInfo {
            repo: "unsloth/gemma-4-26B-A4B-it-GGUF".to_string(),
            filename: "gemma-4-26B-A4B-it-Q4_K_M.gguf".to_string(),
            display_name: "gemma-4-26B-A4B-it-Q4_K_M".to_string(),
            size_hint: "~16 GB",
        },
        "gemma4:26b-q8" => HfModelInfo {
            repo: "unsloth/gemma-4-26B-A4B-it-GGUF".to_string(),
            filename: "gemma-4-26B-A4B-it-Q8_0.gguf".to_string(),
            display_name: "gemma-4-26B-A4B-it-Q8_0".to_string(),
            size_hint: "~28 GB",
        },
        _ => HfModelInfo {
            repo: format!("unsloth/gemma-4-E4B-it-GGUF"),
            filename: "gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
            display_name: alias.to_string(),
            size_hint: "unknown",
        },
    }
}

/// Get the models directory (~/.eunice/models/)
fn models_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".eunice").join("models")
}

/// Find the gemma4-server binary in ~/.eunice/bin/ or PATH
pub fn find_server_binary() -> Option<PathBuf> {
    // Check ~/.eunice/bin/ first
    let home = dirs::home_dir()?;
    let eunice_bin = home.join(".eunice").join("bin").join(SERVER_BINARY_NAME);
    if eunice_bin.exists() {
        return Some(eunice_bin);
    }

    // Check PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg(SERVER_BINARY_NAME)
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }

    None
}

/// Check if an NVIDIA GPU is available
fn has_nvidia_gpu() -> bool {
    std::process::Command::new("nvidia-smi")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Download model weights from HuggingFace
pub async fn download_model(alias: &str) -> Result<PathBuf> {
    let info = resolve_hf_alias(alias);
    let models = models_dir();
    std::fs::create_dir_all(&models)?;

    let dest = models.join(&info.filename);
    if dest.exists() {
        return Ok(dest);
    }

    eprintln!("Downloading {} ({})...", info.filename, info.size_hint);

    let api = hf_hub::api::tokio::Api::new()?;
    let repo = api.model(info.repo);
    let path = repo.get(&info.filename).await
        .map_err(|e| anyhow!("Failed to download {}: {}", info.filename, e))?;

    // hf-hub caches to its own location; copy/link to our models dir
    if path != dest {
        // Try hard link first (same filesystem), fall back to copy
        if std::fs::hard_link(&path, &dest).is_err() {
            std::fs::copy(&path, &dest)?;
        }
    }

    eprintln!("Downloaded to {}", dest.display());
    Ok(dest)
}

/// Start the gemma4-server subprocess
pub fn start_server(model_path: &std::path::Path, port: u16) -> Result<Child> {
    let server_bin = find_server_binary()
        .ok_or_else(|| anyhow!(
            "gemma4-server not found. Install it via:\n  \
             curl -sSf https://longrunningagents.com/install.sh | bash\n  \
             Or download llama-server from https://github.com/ggerganov/llama.cpp/releases \
             and place it at ~/.eunice/bin/gemma4-server"
        ))?;

    let mut cmd = std::process::Command::new(&server_bin);

    // Set LD_LIBRARY_PATH to the server binary's directory (shared libs live alongside it)
    if let Some(bin_dir) = server_bin.parent() {
        let existing = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_path = if existing.is_empty() {
            bin_dir.to_string_lossy().to_string()
        } else {
            format!("{}:{}", bin_dir.to_string_lossy(), existing)
        };
        cmd.env("LD_LIBRARY_PATH", new_path);
    }

    cmd.arg("-m").arg(model_path)
        .arg("--port").arg(port.to_string())
        .arg("--host").arg("127.0.0.1");

    // Enable GPU layers if NVIDIA GPU detected
    if has_nvidia_gpu() {
        cmd.arg("-ngl").arg("999");
    }

    // Suppress server output (it's noisy)
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let child = cmd.spawn()
        .map_err(|e| anyhow!("Failed to start gemma4-server: {}", e))?;

    Ok(child)
}

/// Wait for the server to become ready by polling /health
pub async fn wait_for_ready(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(HEALTH_TIMEOUT_SECS);

    loop {
        if tokio::time::Instant::now() > deadline {
            return Err(anyhow!(
                "gemma4-server failed to start within {} seconds", HEALTH_TIMEOUT_SECS
            ));
        }

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => {}
        }

        tokio::time::sleep(Duration::from_millis(HEALTH_POLL_INTERVAL_MS)).await;
    }
}

/// Full setup: download model if needed, start server, wait for ready
pub async fn setup_local_model(alias: &str) -> Result<(Child, PathBuf)> {
    let model_path = download_model(alias).await?;

    eprint!("Starting gemma4-server...");
    let child = start_server(&model_path, DEFAULT_PORT)?;
    wait_for_ready(DEFAULT_PORT).await?;
    eprintln!(" Ready.");

    Ok((child, model_path))
}

/// List downloaded local models
pub fn list_local_models() -> Result<Vec<(String, u64)>> {
    let models = models_dir();
    if !models.exists() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    for entry in std::fs::read_dir(&models)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "gguf").unwrap_or(false) {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let size = entry.metadata()?.len();
            result.push((name, size));
        }
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}

/// Remove a downloaded model
pub fn remove_model(alias: &str) -> Result<()> {
    let info = resolve_hf_alias(alias);
    let path = models_dir().join(&info.filename);
    if path.exists() {
        std::fs::remove_file(&path)?;
        eprintln!("Removed {}", info.filename);
    } else {
        eprintln!("Model {} not found", info.filename);
    }
    Ok(())
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{} KB", bytes / 1024)
    }
}

/// Print local models list
pub fn print_local_models() -> Result<()> {
    let models = list_local_models()?;
    if models.is_empty() {
        println!("No local models downloaded.");
        println!();
        println!("Download one with:");
        println!("  eunice --download hf:gemma4:e4b");
        return Ok(());
    }

    println!("Local models (~/.eunice/models/):");
    println!();
    for (name, size) in &models {
        println!("  {} ({})", name, format_size(*size));
    }

    let server = find_server_binary();
    println!();
    if let Some(path) = server {
        println!("gemma4-server: {}", path.display());
    } else {
        println!("gemma4-server: not installed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_hf_alias_e4b() {
        let info = resolve_hf_alias("gemma4:e4b");
        assert_eq!(info.repo, "unsloth/gemma-4-E4B-it-GGUF");
        assert_eq!(info.filename, "gemma-4-E4B-it-Q4_K_M.gguf");
        assert_eq!(info.display_name, "gemma-4-E4B-it-Q4_K_M");
    }

    #[test]
    fn test_resolve_hf_alias_e4b_q8() {
        let info = resolve_hf_alias("gemma4:e4b-q8");
        assert_eq!(info.repo, "unsloth/gemma-4-E4B-it-GGUF");
        assert_eq!(info.filename, "gemma-4-E4B-it-Q8_0.gguf");
    }

    #[test]
    fn test_resolve_hf_alias_26b() {
        let info = resolve_hf_alias("gemma4:26b");
        assert_eq!(info.repo, "unsloth/gemma-4-26B-A4B-it-GGUF");
        assert_eq!(info.filename, "gemma-4-26B-A4B-it-Q4_K_M.gguf");
    }

    #[test]
    fn test_resolve_hf_alias_26b_q8() {
        let info = resolve_hf_alias("gemma4:26b-q8");
        assert_eq!(info.repo, "unsloth/gemma-4-26B-A4B-it-GGUF");
        assert_eq!(info.filename, "gemma-4-26B-A4B-it-Q8_0.gguf");
    }

    #[test]
    fn test_resolve_hf_alias_unknown_falls_back() {
        let info = resolve_hf_alias("unknown:model");
        assert_eq!(info.display_name, "unknown:model");
    }

    #[test]
    fn test_models_dir() {
        let dir = models_dir();
        assert!(dir.to_string_lossy().contains(".eunice/models"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
        assert_eq!(format_size(4_831_838_208), "4.5 GB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
        assert_eq!(format_size(1024), "1 KB");
    }

    #[test]
    fn test_list_local_models_empty() {
        // Should not error even if directory doesn't exist
        // (it may or may not exist on the test machine)
        let result = list_local_models();
        assert!(result.is_ok());
    }
}
