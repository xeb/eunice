use anyhow::{anyhow, bail, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 18921;
const SERVER_BINARY_NAME: &str = "gemma4-server";
const HEALTH_TIMEOUT_SECS: u64 = 120;
const HEALTH_POLL_INTERVAL_MS: u64 = 500;

// --- Gemma 4 31B + MTP (auto-built from source) ---
const MTP_REPO: &str = "https://github.com/am17an/llama.cpp.git";
const MTP_BRANCH: &str = "gemma4-mtp";
const MTP_SERVER_LABEL: &str = "gemma4-mtp-server";
const MTP_SRC_DIRNAME: &str = "llama.cpp-mtp";
/// How long to wait for the (large) MTP server to load weights and report healthy.
const MTP_HEALTH_TIMEOUT_SECS: u64 = 600;
/// Default number of draft tokens for MTP speculative decoding.
const MTP_DRAFT_N_MAX: u32 = 4;

/// Resolved HuggingFace model info
#[derive(Default)]
pub struct HfModelInfo {
    pub repo: String,
    pub filename: String,
    pub display_name: String,
    pub size_hint: &'static str,
    /// HuggingFace repo for the MTP drafter (speculative decoding), if any.
    pub drafter_repo: Option<String>,
    /// Drafter GGUF filename, if any.
    pub drafter_filename: Option<String>,
    /// Whether this model uses Multi-Token Prediction (requires the auto-built MTP server).
    pub mtp: bool,
    /// Context window to pass to the server (0 = server default). 8192 for 31B avoids OOM.
    pub ctx: u32,
    /// Minimum VRAM (GB) needed to run MTP on-GPU; below this we fall back to plain decode.
    pub min_vram_gb: u32,
}

/// Paths to the local GGUF weights (target model + optional MTP drafter).
pub struct LocalModelPaths {
    pub model: PathBuf,
    pub drafter: Option<PathBuf>,
}

/// Resolve a model alias like "gemma4:e4b" to HuggingFace coordinates
pub fn resolve_hf_alias(alias: &str) -> HfModelInfo {
    match alias {
        "gemma4:e4b" | "gemma4:e4b-q4" => HfModelInfo {
            repo: "unsloth/gemma-4-E4B-it-GGUF".to_string(),
            filename: "gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
            display_name: "gemma-4-E4B-it-Q4_K_M".to_string(),
            size_hint: "~4.5 GB",
            ..Default::default()
        },
        "gemma4:e4b-q8" => HfModelInfo {
            repo: "unsloth/gemma-4-E4B-it-GGUF".to_string(),
            filename: "gemma-4-E4B-it-Q8_0.gguf".to_string(),
            display_name: "gemma-4-E4B-it-Q8_0".to_string(),
            size_hint: "~8 GB",
            ..Default::default()
        },
        "gemma4:e4b-q5" => HfModelInfo {
            repo: "unsloth/gemma-4-E4B-it-GGUF".to_string(),
            filename: "gemma-4-E4B-it-Q5_K_M.gguf".to_string(),
            display_name: "gemma-4-E4B-it-Q5_K_M".to_string(),
            size_hint: "~5.5 GB",
            ..Default::default()
        },
        "gemma4:26b" | "gemma4:26b-q4" => HfModelInfo {
            repo: "unsloth/gemma-4-26B-A4B-it-GGUF".to_string(),
            filename: "gemma-4-26B-A4B-it-Q4_K_M.gguf".to_string(),
            display_name: "gemma-4-26B-A4B-it-Q4_K_M".to_string(),
            size_hint: "~16 GB",
            ..Default::default()
        },
        "gemma4:26b-q8" => HfModelInfo {
            repo: "unsloth/gemma-4-26B-A4B-it-GGUF".to_string(),
            filename: "gemma-4-26B-A4B-it-Q8_0.gguf".to_string(),
            display_name: "gemma-4-26B-A4B-it-Q8_0".to_string(),
            size_hint: "~28 GB",
            ..Default::default()
        },
        // Gemma 4 31B with Multi-Token Prediction — auto-built MTP server + matched drafter.
        "gemma4:31b" | "gemma4:31b-mtp" => HfModelInfo {
            repo: "bartowski/google_gemma-4-31B-it-GGUF".to_string(),
            filename: "google_gemma-4-31B-it-Q4_K_M.gguf".to_string(),
            display_name: "gemma-4-31B-it-Q4_K_M (MTP)".to_string(),
            size_hint: "~19 GB",
            drafter_repo: Some("am17an/Gemma4-31B-it-GGUF".to_string()),
            drafter_filename: Some("mtp-gemma-4-31B-it.gguf".to_string()),
            mtp: true,
            ctx: 8192,
            min_vram_gb: 24,
        },
        _ => HfModelInfo {
            repo: "unsloth/gemma-4-E4B-it-GGUF".to_string(),
            filename: "gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
            display_name: alias.to_string(),
            size_hint: "unknown",
            ..Default::default()
        },
    }
}

/// The ~/.eunice base directory
fn eunice_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".eunice")
}

/// Get the models directory (~/.eunice/models/)
fn models_dir() -> PathBuf {
    eunice_dir().join("models")
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

/// Download a single GGUF file from HuggingFace into ~/.eunice/models/ (idempotent, resumable).
async fn download_gguf(repo: &str, filename: &str, size_hint: &str) -> Result<PathBuf> {
    let models = models_dir();
    std::fs::create_dir_all(&models)?;

    let dest = models.join(filename);
    if dest.exists() {
        return Ok(dest);
    }

    eprintln!("Downloading {} ({})...", filename, size_hint);

    let api = hf_hub::api::tokio::Api::new()?;
    let repo_handle = api.model(repo.to_string());
    let path = repo_handle
        .get(filename)
        .await
        .map_err(|e| anyhow!("Failed to download {}: {}", filename, e))?;

    // hf-hub caches to its own location; link (same FS) or copy (cross-FS) into our models dir.
    if path != dest {
        if std::fs::hard_link(&path, &dest).is_err() {
            std::fs::copy(&path, &dest)?;
        }
    }

    eprintln!("Downloaded to {}", dest.display());
    Ok(dest)
}

/// Download model weights from HuggingFace (target GGUF only)
pub async fn download_model(alias: &str) -> Result<PathBuf> {
    let info = resolve_hf_alias(alias);
    download_gguf(&info.repo, &info.filename, info.size_hint).await
}

/// Download the target model and (if applicable) the MTP drafter.
pub async fn download_model_full(alias: &str) -> Result<LocalModelPaths> {
    let info = resolve_hf_alias(alias);
    let model = download_gguf(&info.repo, &info.filename, info.size_hint).await?;
    let drafter = match (info.drafter_repo.as_ref(), info.drafter_filename.as_ref()) {
        (Some(repo), Some(file)) => Some(download_gguf(repo, file, "~0.5 GB drafter").await?),
        _ => None,
    };
    Ok(LocalModelPaths { model, drafter })
}

/// Start the (stock) gemma4-server subprocess for non-MTP local models (E4B / 26B).
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

/// Path to the server's log file (MTP path captures stdout/stderr here for diagnostics).
fn mtp_log_path() -> PathBuf {
    eunice_dir().join("gemma4-mtp-server.log")
}

/// Start the auto-built MTP server with speculative-decoding flags.
pub fn start_mtp_server(
    server_bin: &Path,
    paths: &LocalModelPaths,
    port: u16,
    ctx: u32,
    n_max: u32,
    run_mtp: bool,
) -> Result<Child> {
    let mut cmd = Command::new(server_bin);

    // The built binary is a thin loader; its shared libs live next to it.
    if let Some(bin_dir) = server_bin.parent() {
        let existing = env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_path = if existing.is_empty() {
            bin_dir.to_string_lossy().to_string()
        } else {
            format!("{}:{}", bin_dir.to_string_lossy(), existing)
        };
        cmd.env("LD_LIBRARY_PATH", new_path);
    }

    cmd.arg("-m").arg(&paths.model)
        .arg("--host").arg("127.0.0.1")
        .arg("--port").arg(port.to_string())
        .arg("-ngl").arg("999");

    if ctx > 0 {
        cmd.arg("-c").arg(ctx.to_string());
        // Quantize the KV cache (q8_0) so target + draft + KV fit in 24 GB at this context.
        // llama-server has more overhead than llama-cli; f16 KV at ctx=8192 OOMs by ~0.5 GB.
        cmd.arg("-ctk").arg("q8_0").arg("-ctv").arg("q8_0");
    }

    if run_mtp {
        if let Some(drafter) = &paths.drafter {
            cmd.arg("-md").arg(drafter)
                .arg("-ngld").arg("999")
                .arg("-fa").arg("on")
                .arg("--spec-type").arg("draft-mtp")
                .arg("--spec-draft-n-max").arg(n_max.to_string());
        } else {
            eprintln!("⚠ No MTP drafter present — running 31B without MTP.");
        }
    } else {
        eprintln!("⚠ Insufficient VRAM for MTP — running plain 31B (~42 tok/s).");
    }

    // Apply the Gemma 4 chat template so tool/function calling works through eunice.
    // (--jinja is default-enabled on this build; passed explicitly as belt-and-suspenders.)
    cmd.arg("--jinja");

    // Log server output for verification/troubleshooting (MTP activation, errors, tok/s).
    match std::fs::File::create(mtp_log_path()) {
        Ok(f) => {
            let f2 = f.try_clone();
            cmd.stdout(Stdio::from(f));
            match f2 {
                Ok(f2) => {
                    cmd.stderr(Stdio::from(f2));
                }
                Err(_) => {
                    cmd.stderr(Stdio::null());
                }
            }
        }
        Err(_) => {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }

    let child = cmd
        .spawn()
        .map_err(|e| anyhow!("Failed to start {}: {}", MTP_SERVER_LABEL, e))?;

    Ok(child)
}

/// Is something already answering /health on this port?
async fn port_healthy(port: u16) -> bool {
    reqwest::Client::new()
        .get(format!("http://127.0.0.1:{}/health", port))
        .timeout(Duration::from_millis(500))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Wait for the server to become ready by polling /health (default timeout).
pub async fn wait_for_ready(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(HEALTH_TIMEOUT_SECS);

    loop {
        if tokio::time::Instant::now() > deadline {
            return Err(anyhow!(
                "server failed to become ready within {} seconds", HEALTH_TIMEOUT_SECS
            ));
        }

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => {}
        }

        tokio::time::sleep(Duration::from_millis(HEALTH_POLL_INTERVAL_MS)).await;
    }
}

/// Wait for readiness while also detecting an early crash of the spawned child.
/// On crash, surfaces the tail of the server log instead of a generic timeout.
pub async fn wait_for_ready_or_exit(child: &mut Child, port: u16, timeout_secs: u64) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        // Did the server process die (OOM, missing lib, bad flag, port in use)?
        if let Ok(Some(status)) = child.try_wait() {
            bail!(
                "{} exited before becoming ready ({}).\n{}\n(full log: {})",
                MTP_SERVER_LABEL,
                status,
                log_tail(15),
                mtp_log_path().display()
            );
        }

        if tokio::time::Instant::now() > deadline {
            bail!(
                "{} did not become ready within {}s.\n{}\n(full log: {})",
                MTP_SERVER_LABEL,
                timeout_secs,
                log_tail(15),
                mtp_log_path().display()
            );
        }

        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }

        tokio::time::sleep(Duration::from_millis(HEALTH_POLL_INTERVAL_MS)).await;
    }
}

/// Last `n` lines of the MTP server log (for diagnostics).
fn log_tail(n: usize) -> String {
    std::fs::read_to_string(mtp_log_path())
        .ok()
        .map(|s| {
            let lines: Vec<&str> = s.lines().collect();
            let start = lines.len().saturating_sub(n);
            lines[start..].join("\n")
        })
        .unwrap_or_default()
}

/// Full setup: validate (MTP), download model(s), build (MTP) if needed, start server, wait ready.
pub async fn setup_local_model(alias: &str) -> Result<(Child, PathBuf)> {
    setup_local_model_on_port(alias, DEFAULT_PORT).await
}

/// Same as [`setup_local_model`] but on a specific port (used by `--serve`).
pub async fn setup_local_model_on_port(alias: &str, port: u16) -> Result<(Child, PathBuf)> {
    let info = resolve_hf_alias(alias);

    if !info.mtp {
        // Existing E4B / 26B path — unchanged behavior.
        let model_path = download_model(alias).await?;
        eprint!("Starting gemma4-server...");
        let child = start_server(&model_path, port)?;
        wait_for_ready(port).await?;
        eprintln!(" Ready.");
        return Ok((child, model_path));
    }

    // --- Gemma 4 31B + MTP path ---

    // 1. Validation gate — runs before any download or build.
    let pf = preflight_gemma4_mtp(&info);
    print_preflight(&pf);
    if !pf.ok() {
        bail!(
            "gemma4:31b prerequisites not met — fix the ✗ items above and retry. \
             (nothing was downloaded or built)"
        );
    }

    // 2. Download target + drafter.
    let paths = download_model_full(alias).await?;

    // 3. Ensure the MTP-capable server is built (cached after first run).
    let arch = pf.cuda_arch.clone().unwrap_or_else(|| "89".to_string());
    let server_bin = ensure_mtp_server(&arch, false).await?;

    // 4. Guard against a stale server already holding the port.
    if port_healthy(port).await {
        bail!(
            "a server is already responding on port {} — stop it (or it may be another eunice) and retry.",
            port
        );
    }

    // 5. Start the server and wait, detecting early crashes.
    eprint!("Starting {} (loading weights, may take a minute)...", MTP_SERVER_LABEL);
    let mut child = start_mtp_server(&server_bin, &paths, port, info.ctx, MTP_DRAFT_N_MAX, pf.run_mtp)?;
    if let Err(e) = wait_for_ready_or_exit(&mut child, port, MTP_HEALTH_TIMEOUT_SECS).await {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }
    eprintln!(" Ready.");

    Ok((child, paths.model))
}

// =====================================================================================
// Validation gate (preflight) for gemma4:31b + MTP
// =====================================================================================

/// A single preflight check result.
pub struct Check {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
    pub hard: bool,
}

/// Aggregated preflight results.
pub struct Preflight {
    pub checks: Vec<Check>,
    pub cuda_arch: Option<String>,
    /// Whether there is enough VRAM to actually enable MTP (else fall back to plain decode).
    pub run_mtp: bool,
}

impl Preflight {
    pub fn hard_failures(&self) -> Vec<&Check> {
        self.checks.iter().filter(|c| c.hard && !c.ok).collect()
    }
    pub fn ok(&self) -> bool {
        self.hard_failures().is_empty()
    }
}

/// Run a command and report success + first line of output.
fn check_cmd(name: &'static str, args: &[&str], hard: bool) -> Check {
    match Command::new(name).args(args).output() {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let mut line = stdout.lines().next().unwrap_or("").trim().to_string();
            if line.is_empty() {
                let stderr = String::from_utf8_lossy(&o.stderr);
                line = stderr.lines().next().unwrap_or("").trim().to_string();
            }
            Check { name, ok: true, detail: line, hard }
        }
        _ => Check { name, ok: false, detail: "not found".to_string(), hard },
    }
}

/// Parse "X.Y..." into (major, minor).
fn parse_major_minor(v: &str) -> (u32, u32) {
    let mut it = v.split('.');
    let maj = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let min = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    (maj, min)
}

/// Check that cmake exists and meets a minimum version.
fn check_cmake_min(min: &str) -> Check {
    match Command::new("cmake").arg("--version").output() {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            let ver = s
                .lines()
                .next()
                .unwrap_or("")
                .split_whitespace()
                .last()
                .unwrap_or("0")
                .to_string();
            let (vmaj, vmin) = parse_major_minor(&ver);
            let (mmaj, mmin) = parse_major_minor(min);
            let ok = vmaj > mmaj || (vmaj == mmaj && vmin >= mmin);
            Check {
                name: "cmake",
                ok,
                detail: format!("{} (need >= {})", ver, min),
                hard: true,
            }
        }
        _ => Check {
            name: "cmake",
            ok: false,
            detail: "not found (need >= 3.18)".to_string(),
            hard: true,
        },
    }
}

/// Detect the GPU CUDA compute capability as a CMake arch string (e.g. "8.9" -> "89").
fn detect_cuda_arch() -> Option<String> {
    let o = Command::new("nvidia-smi")
        .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&o.stdout);
    let first = s.lines().next()?.trim();
    if first.is_empty() {
        return None;
    }
    Some(first.replace('.', ""))
}

/// Total GPU VRAM in GB (rounded), or 0 if undetectable.
fn gpu_vram_gb() -> u32 {
    Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .map(|l| l.trim().to_string())
        })
        .and_then(|s| s.parse::<f64>().ok())
        .map(|mib| (mib / 1024.0).round() as u32)
        .unwrap_or(0)
}

/// Free space (GB) on the filesystem backing `path` (walks up to the nearest existing ancestor).
fn free_space_gb(path: &Path) -> u64 {
    let mut p = path.to_path_buf();
    while !p.exists() {
        match p.parent() {
            Some(par) => p = par.to_path_buf(),
            None => break,
        }
    }
    Command::new("df")
        .arg("-Pk")
        .arg(&p)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            let line = s.lines().nth(1)?;
            let avail_kb: u64 = line.split_whitespace().nth(3)?.parse().ok()?;
            Some(avail_kb / 1024 / 1024)
        })
        .unwrap_or(0)
}

/// Disk space (GB) still required, subtracting anything already cached.
fn required_disk_gb(info: &HfModelInfo) -> u64 {
    let mut gb: u64 = 35; // ~19 GB model + 0.5 GB drafter + ~0.7 GB source + ~6-8 GB build artifacts + headroom
    if models_dir().join(&info.filename).exists() {
        gb = gb.saturating_sub(20);
    }
    if mtp_server_installed() {
        gb = gb.saturating_sub(12);
    }
    gb.max(1)
}

/// Validate everything needed for gemma4:31b + MTP. Pure inspection — no downloads, no builds.
pub fn preflight_gemma4_mtp(info: &HfModelInfo) -> Preflight {
    let mut checks = Vec::new();

    checks.push(check_cmd("git", &["--version"], true));
    checks.push(check_cmake_min("3.18"));
    checks.push(check_cmd("c++", &["--version"], true));
    checks.push(check_cmd("nvcc", &["--version"], true));
    checks.push(check_cmd("nvidia-smi", &["-L"], true));

    // Disk: only count what we still need to fetch/build.
    let need_gb = required_disk_gb(info);
    let free_gb = free_space_gb(&eunice_dir());
    checks.push(Check {
        name: "disk",
        ok: free_gb >= need_gb,
        detail: format!("{} GB free, need {} GB in {}", free_gb, need_gb, eunice_dir().display()),
        hard: true,
    });

    // VRAM gates MTP itself (soft) — below threshold we run plain 31B.
    let vram = gpu_vram_gb();
    let run_mtp = vram >= info.min_vram_gb;
    checks.push(Check {
        name: "vram",
        ok: run_mtp,
        detail: format!("{} GB (need {} for MTP; else plain 31B)", vram, info.min_vram_gb),
        hard: false,
    });

    Preflight {
        checks,
        cuda_arch: detect_cuda_arch(),
        run_mtp,
    }
}

/// Print the preflight report (✓ / ✗ / ⚠).
pub fn print_preflight(pf: &Preflight) {
    eprintln!("Checking prerequisites for gemma4:31b + MTP…");
    for c in &pf.checks {
        let mark = if c.ok { "✓" } else if c.hard { "✗" } else { "⚠" };
        eprintln!("  {} {:<11} {}", mark, c.name, c.detail);
    }
    eprintln!();
}

// =====================================================================================
// Auto-build of the MTP-capable llama-server
// =====================================================================================
//
// The built binary stays IN its build tree (build/bin/llama-server) where its shared libs are
// co-located — we do NOT copy it into ~/.eunice/bin/ (that would collide with the stock
// gemma4-server's older libs). LD_LIBRARY_PATH points at the build/bin dir at launch.

fn mtp_build_root() -> PathBuf {
    eunice_dir().join("build")
}

fn mtp_src_dir() -> PathBuf {
    mtp_build_root().join(MTP_SRC_DIRNAME)
}

fn mtp_server_path() -> PathBuf {
    mtp_src_dir().join("build").join("bin").join("llama-server")
}

fn mtp_version_path() -> PathBuf {
    mtp_build_root().join(".gemma4-mtp.version")
}

/// Whether an MTP server is available (built in-tree, or pointed at via EUNICE_GEMMA4_SERVER).
pub fn mtp_server_installed() -> bool {
    if let Ok(p) = env::var("EUNICE_GEMMA4_SERVER") {
        if Path::new(&p).exists() {
            return true;
        }
    }
    mtp_server_path().exists()
}

/// The git ref (branch or commit) to build; overridable via EUNICE_GEMMA4_MTP_REF.
fn desired_ref() -> String {
    env::var("EUNICE_GEMMA4_MTP_REF").unwrap_or_else(|_| MTP_BRANCH.to_string())
}

fn version_matches(want: &str) -> bool {
    std::fs::read_to_string(mtp_version_path())
        .ok()
        .and_then(|s| s.lines().next().map(|l| l.trim() == want))
        .unwrap_or(false)
}

fn write_version(want: &str, arch: &str) -> Result<()> {
    if let Some(parent) = mtp_version_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(mtp_version_path(), format!("{}\n{}\n", want, arch))?;
    Ok(())
}

/// Exclusive build lock so two eunice processes don't build concurrently.
struct BuildLock(PathBuf);

impl BuildLock {
    fn acquire(build_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(build_dir)?;
        let lock = build_dir.join(".build.lock");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
        {
            Ok(_) => Ok(BuildLock(lock)),
            Err(_) => bail!(
                "a gemma4-mtp build appears to be in progress (lock at {}). \
                 If no build is running, delete that file and retry.",
                lock.display()
            ),
        }
    }
}

impl Drop for BuildLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Clone the MTP branch (shallow) if not already present.
fn clone_if_needed(src: &Path, reference: &str) -> Result<()> {
    if src.join(".git").exists() {
        return Ok(());
    }
    if let Some(parent) = src.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("git")
        .args(["clone", "--depth", "1", "--branch", reference, MTP_REPO])
        .arg(src)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        bail!("git clone of {} (branch {}) failed", MTP_REPO, reference);
    }
    Ok(())
}

/// Run a build step in `dir`, streaming output live.
fn run_build_step(dir: &Path, cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| anyhow!("failed to run `{}`: {}", cmd, e))?;
    if !status.success() {
        bail!("`{} {}` failed (see output above)", cmd, args.join(" "));
    }
    Ok(())
}

/// Ensure the MTP-capable server binary exists; build it from source if needed.
/// Returns the path to the server binary. Idempotent + lock-guarded.
pub async fn ensure_mtp_server(arch: &str, force: bool) -> Result<PathBuf> {
    // Escape hatch: point at an externally-built server (used for dev/testing).
    if let Ok(p) = env::var("EUNICE_GEMMA4_SERVER") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            eprintln!("Using EUNICE_GEMMA4_SERVER={}", p);
            return Ok(pb);
        }
        eprintln!("⚠ EUNICE_GEMMA4_SERVER set to {} but not found — building from source.", p);
    }

    let want = desired_ref();
    let bin = mtp_server_path();
    if !force && bin.exists() && version_matches(&want) {
        return Ok(bin);
    }

    let _lock = BuildLock::acquire(&mtp_build_root())?;
    let src = mtp_src_dir();

    eprintln!(
        "Building {} from {} (branch {}). First run only — this takes ~10–15 min.",
        MTP_SERVER_LABEL, MTP_REPO, want
    );

    clone_if_needed(&src, &want)?;

    let jobs = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .to_string();

    run_build_step(
        &src,
        "cmake",
        &[
            "-B",
            "build",
            "-DGGML_CUDA=ON",
            &format!("-DCMAKE_CUDA_ARCHITECTURES={}", arch),
            "-DLLAMA_CURL=OFF",
            "-DCMAKE_BUILD_TYPE=Release",
        ],
    )?;
    run_build_step(
        &src,
        "cmake",
        &["--build", "build", "--config", "Release", "-j", &jobs, "--target", "llama-server"],
    )?;

    if !bin.exists() {
        bail!("build completed but {} was not produced", bin.display());
    }

    write_version(&want, arch)?;
    eprintln!("✓ Built {}", bin.display());
    Ok(bin)
}

/// Force a clean rebuild of the MTP server (used by `--rebuild-gemma4-mtp`).
pub async fn rebuild_mtp_server() -> Result<PathBuf> {
    let arch = detect_cuda_arch().unwrap_or_else(|| "89".to_string());
    // Remove the cached source so we re-clone the latest of the desired ref.
    let src = mtp_src_dir();
    if src.exists() {
        let _ = std::fs::remove_dir_all(&src);
    }
    let _ = std::fs::remove_file(mtp_version_path());
    ensure_mtp_server(&arch, true).await
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
            let size = std::fs::metadata(&path)?.len(); // follows symlinks (pre-seeded test weights)
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

    let mtp_bin = mtp_server_path();
    if mtp_bin.exists() {
        println!("gemma4-mtp-server: {}", mtp_bin.display());
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
        assert!(!info.mtp);
        assert!(info.drafter_repo.is_none());
        assert_eq!(info.ctx, 0);
    }

    #[test]
    fn test_resolve_hf_alias_e4b_q8() {
        let info = resolve_hf_alias("gemma4:e4b-q8");
        assert_eq!(info.repo, "unsloth/gemma-4-E4B-it-GGUF");
        assert_eq!(info.filename, "gemma-4-E4B-it-Q8_0.gguf");
        assert!(!info.mtp);
    }

    #[test]
    fn test_resolve_hf_alias_26b() {
        let info = resolve_hf_alias("gemma4:26b");
        assert_eq!(info.repo, "unsloth/gemma-4-26B-A4B-it-GGUF");
        assert_eq!(info.filename, "gemma-4-26B-A4B-it-Q4_K_M.gguf");
        assert!(!info.mtp);
    }

    #[test]
    fn test_resolve_hf_alias_26b_q8() {
        let info = resolve_hf_alias("gemma4:26b-q8");
        assert_eq!(info.repo, "unsloth/gemma-4-26B-A4B-it-GGUF");
        assert_eq!(info.filename, "gemma-4-26B-A4B-it-Q8_0.gguf");
    }

    #[test]
    fn test_resolve_hf_alias_31b_mtp() {
        for alias in ["gemma4:31b", "gemma4:31b-mtp"] {
            let info = resolve_hf_alias(alias);
            assert!(info.mtp, "{} should be mtp", alias);
            assert_eq!(info.ctx, 8192);
            assert_eq!(info.min_vram_gb, 24);
            assert_eq!(info.repo, "bartowski/google_gemma-4-31B-it-GGUF");
            assert_eq!(info.filename, "google_gemma-4-31B-it-Q4_K_M.gguf");
            assert_eq!(info.drafter_repo.as_deref(), Some("am17an/Gemma4-31B-it-GGUF"));
            assert_eq!(info.drafter_filename.as_deref(), Some("mtp-gemma-4-31B-it.gguf"));
        }
    }

    #[test]
    fn test_resolve_hf_alias_unknown_falls_back() {
        let info = resolve_hf_alias("unknown:model");
        assert_eq!(info.display_name, "unknown:model");
        assert!(!info.mtp);
    }

    #[test]
    fn test_existing_aliases_not_mtp() {
        for alias in ["gemma4:e4b", "gemma4:e4b-q8", "gemma4:e4b-q5", "gemma4:26b", "gemma4:26b-q8", "weird:thing"] {
            let info = resolve_hf_alias(alias);
            assert!(!info.mtp, "{} must not be mtp", alias);
            assert_eq!(info.ctx, 0, "{} ctx must be 0", alias);
            assert!(info.drafter_repo.is_none(), "{} must have no drafter", alias);
        }
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
        let result = list_local_models();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_major_minor() {
        assert_eq!(parse_major_minor("3.22.1"), (3, 22));
        assert_eq!(parse_major_minor("3.18"), (3, 18));
        assert_eq!(parse_major_minor("12"), (12, 0));
    }

    #[test]
    fn test_required_disk_gb_is_bounded() {
        let info = resolve_hf_alias("gemma4:31b");
        let gb = required_disk_gb(&info);
        assert!(gb >= 1 && gb <= 35);
    }

    #[test]
    fn test_preflight_includes_core_checks() {
        let info = resolve_hf_alias("gemma4:31b");
        let pf = preflight_gemma4_mtp(&info);
        for name in ["git", "cmake", "c++", "nvcc", "nvidia-smi", "disk", "vram"] {
            assert!(pf.checks.iter().any(|c| c.name == name), "missing check: {}", name);
        }
        assert_eq!(pf.ok(), pf.hard_failures().is_empty());
    }
}
