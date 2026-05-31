# Gemma 4 31B + MTP (≈96 tok/s) inside Eunice — Auto Build & Run Plan

> **Note on the name:** the file says "32B" but the model is **Gemma 4 31B** (`google/gemma-4-31B-it`,
> 30.7B params). There is no 32B. This plan covers the large dense 31B model accelerated with
> **Multi-Token Prediction (MTP)** speculative decoding via pure `llama.cpp`.

## Goal

`eunice --model=gemma4:31b "..."` should **just work** with zero manual setup: on first use eunice
**auto-downloads the model + drafter and auto-builds the MTP-capable `llama.cpp` server from source
(CUDA)**, installs everything under `~/.eunice/`, then runs it via the existing `Provider::Local`
plumbing (local server → OpenAI-compatible client). Subsequent runs reuse the cached binary/weights
and start in seconds.

## Why a *build* (not just a download)

Gemma 4 MTP is **not in any released `llama.cpp` binary**. It lives in unmerged **WIP PR #23398**
(`am17an/llama.cpp`, branch `gemma4-mtp`). Stock releases reject the drafter with
`unknown model architecture: 'gemma4_assistant'`. So to get the ≈2.3× speedup we must **compile that
branch with CUDA on the user's machine** (architecture-matched). This is the price of being on the
bleeding edge; when the PR merges to a tagged release we can switch to a plain binary download
(see "Future" below).

## Measured result on THIS computer (RTX 4090, 24 GB)

| Setup | tok/s |
|---|---|
| llama.cpp baseline (no MTP) | 41.8 |
| **llama.cpp + MTP** | **96.7** (≈2.31×, lossless — output bit-identical) |

Reference build/repro: `/media/xeb/GreyArea/projects/gemmafun/GEMMA4_MTP_LLAMACPP.md`.

---

## ✅ Implemented & verified (2026-05-29)

Implemented in `src/local.rs`, `src/provider.rs`, `src/main.rs`. `cargo test` green (142 tests).
End-to-end on the RTX 4090 via the cache-hit path:

- **Routing:** `eunice --model=gemma4:31b` → `Provider::Local`; bare `gemma4:e4b`/`gemma4:26b` still go to Ollama.
- **Preflight gate** prints ✓/✗ for git/cmake/c++/nvcc/nvidia-smi/disk/vram; a missing `nvcc` aborts fast with *"prerequisites not met (nothing was downloaded or built)"* and never starts a server.
- **Run:** server starts, answers; **tool-loop works** (model emits a `Bash` tool call, eunice runs it, model reports the result).
- **Throughput:** **76 tok/s** via the OpenAI endpoint with `draft_n=359, accepted=209` (~58% accept) vs ~42 baseline — MTP genuinely active (`adding speculative implementation 'draft-mtp'`).

Two deltas discovered during testing and folded into the implementation:
1. **Binary stays in its build tree** (`~/.eunice/build/llama.cpp-mtp/build/bin/llama-server`), not copied to `~/.eunice/bin/` — that dir already holds the stock `gemma4-server`'s older `.so`s (0.9.11 vs MTP's 0.13.0) and co-locating would collide. `LD_LIBRARY_PATH` points at the build/bin dir.
2. **KV cache is quantized (`-ctk q8_0 -ctv q8_0`)** — `llama-server` has more overhead than `llama-cli`, so f16 KV at `-c 8192` OOMs by ~0.5 GB; q8 KV fits with margin and keeps the full 8192 window.

Plus robustness: early-crash detection (surfaces the server-log tail instead of a 600s hang) and a port-in-use pre-check.

---

## Target layout under `~/.eunice/`

```
~/.eunice/
├── build/
│   ├── llama.cpp-mtp/                     # cloned source; the built binary lives at
│   │   └── build/bin/llama-server         #   …/build/bin/llama-server (+ its .so siblings)
│   └── .gemma4-mtp.version                # built ref + CUDA arch (cache invalidation)
├── gemma4-mtp-server.log                  # server stdout/stderr (MTP activation, errors)
└── models/
    ├── google_gemma-4-31B-it-Q4_K_M.gguf # target (~19 GB)
    └── mtp-gemma-4-31B-it.gguf           # MTP drafter (~491 MB)

# NOTE: the binary is NOT copied into ~/.eunice/bin/ — that dir holds the stock gemma4-server's
# older .so set; the MTP binary runs in-place with LD_LIBRARY_PATH set to its build/bin dir.
```

---

## End-to-end flow for `--model=gemma4:31b`

```
eunice --model=gemma4:31b "write a python LRU cache"
  │
  1. Routing: provider.rs maps gemma4:31b (and hf:gemma4:31b) → Provider::Local, mtp=true
  2. preflight_gemma4_mtp():   ← VALIDATION GATE (runs FIRST, before any download or build)
       • aggregates ALL checks (deps + GPU + VRAM + disk), then decides
       • if everything cached (binary version matches + both GGUFs present) → skip to step 5
       • hard failure → print full report, abort BEFORE downloading/building anything
       • soft failure (e.g. <24 GB VRAM) → warn, mark "run without MTP"
  3. ensure_models():   download target + drafter GGUF → ~/.eunice/models/  (hf-hub, resumable)
  4. ensure_mtp_server():   (only the missing/stale steps; arch already detected in preflight)
       clone --depth 1 --branch gemma4-mtp → cmake (CUDA) → build llama-server → install to bin/
  5. start_server(): spawn gemma4-mtp-server with MTP flags (below), poll /health
  6. agent loop talks to 127.0.0.1:18921/v1/  → ≈96 tok/s
  7. on exit: kill server subprocess
```

**Validation is a single up-front gate (step 2).** Nothing is downloaded or built until every hard
requirement passes — so a user missing `nvcc` finds out in <1 s, not after a 19 GB download. First
run ≈ 10–15 min (CUDA compile) + the 19 GB download. Every run after: seconds.

---

## Code changes

### 1. Routing (`src/provider.rs`)

`gemma4:31b` has no working Ollama route (the Ollama `gemma4:31b-coding-mtp-bf16` tag is macOS-gated),
so route it to the local MTP path. Accept both `gemma4:31b` and `hf:gemma4:31b`:

```rust
// in detect_provider(), before the generic Ollama fallthrough:
let local_alias = model.strip_prefix("hf:").unwrap_or(model);
if local_alias == "gemma4:31b" || local_alias.starts_with("gemma4:31b") {
    let resolved = crate::local::resolve_hf_alias(local_alias);
    return ProviderInfo {
        provider: Provider::Local,
        base_url: format!("http://127.0.0.1:{}/v1/", crate::local::DEFAULT_PORT),
        api_key: "local".to_string(),
        model: resolved.display_name,
        ..
    };
}
```
(The existing `hf:`-prefix branch at `provider.rs:184` already handles E4B/26B; this just lets the
bare `gemma4:31b` resolve locally too.) Add it to the `--list-models` local list (~line 356).

### 2. Model descriptor + alias (`src/local.rs`)

```rust
pub struct HfModelInfo {
    pub repo: String,
    pub filename: String,
    pub display_name: String,
    pub size_hint: &'static str,
    // MTP additions:
    pub drafter_repo: Option<String>,
    pub drafter_filename: Option<String>,
    pub mtp: bool,
    pub ctx: u32,          // 8192 for 31B (mandatory — see OOM note)
    pub min_vram_gb: u32,  // 24 for 31B
}

"gemma4:31b" | "gemma4:31b-mtp" => HfModelInfo {
    repo: "bartowski/google_gemma-4-31B-it-GGUF".into(),
    filename: "google_gemma-4-31B-it-Q4_K_M.gguf".into(),
    display_name: "gemma-4-31B-it-Q4_K_M (MTP)".into(),
    size_hint: "~19 GB + 0.5 GB drafter",
    drafter_repo: Some("am17an/Gemma4-31B-it-GGUF".into()),
    drafter_filename: Some("mtp-gemma-4-31B-it.gguf".into()),
    mtp: true, ctx: 8192, min_vram_gb: 24,
},
```
Existing aliases set the new fields to `None/false/0`. `download_model` gains a drafter fetch and
returns `LocalModelPaths { model: PathBuf, drafter: Option<PathBuf> }`.

### 3. Auto-build (`src/local.rs` — new)

```rust
const MTP_REPO: &str   = "https://github.com/am17an/llama.cpp.git";
const MTP_BRANCH: &str = "gemma4-mtp";
const MTP_SERVER: &str = "gemma4-mtp-server";

/// Ensure the MTP-capable server is built; returns its path. Idempotent + lock-guarded.
/// `arch` comes from the already-passed validation gate (preflight_gemma4_mtp).
pub async fn ensure_mtp_server(arch: &str) -> Result<PathBuf> {
    let bin = eunice_dir().join("bin").join(MTP_SERVER);
    let want = desired_ref();                 // env EUNICE_GEMMA4_MTP_REF or pinned default SHA
    if bin.exists() && version_matches(&bin, &want)? { return Ok(bin); }

    // Deps/disk/GPU were already validated up front by preflight_gemma4_mtp() — no re-check here.
    let _lock = build_lock()?;                // flock so two eunices don't build at once

    let src = eunice_dir().join("build").join("llama.cpp-mtp");
    clone_or_update(&src, MTP_REPO, MTP_BRANCH, &want)?;     // git clone --depth 1 --branch …

    run_streaming(&src, "cmake", &[
        "-B","build","-DGGML_CUDA=ON",
        &format!("-DCMAKE_CUDA_ARCHITECTURES={arch}"),
        "-DBUILD_SHARED_LIBS=OFF","-DLLAMA_CURL=OFF","-DCMAKE_BUILD_TYPE=Release",
    ])?;
    run_streaming(&src, "cmake", &["--build","build","-j", "--target","llama-server"])?;

    std::fs::create_dir_all(bin.parent().unwrap())?;
    std::fs::copy(src.join("build/bin/llama-server"), &bin)?;
    write_version(&bin, &want, &arch)?;
    Ok(bin)
}
```
- `-DBUILD_SHARED_LIBS=OFF` → a self-contained `llama-server` (only the CUDA runtime stays dynamic),
  so we avoid the `LD_LIBRARY_PATH` shuffle. If shared libs are produced anyway, copy `build/bin/*.so`
  next to the binary (the existing `start_server` already sets `LD_LIBRARY_PATH` to the bin dir).
- `-DLLAMA_CURL=OFF` avoids a libcurl build dependency we don't need (eunice downloads via hf-hub).
- Progress UX: tail `build.log`; print `Building gemma4-mtp-server (first run, ~10–15 min)…` with a
  spinner, matching eunice's existing "Starting gemma4-server…" style.

### 4. MTP-aware `start_server` (`src/local.rs`)

```rust
cmd.arg("-m").arg(&paths.model)
   .arg("--host").arg("127.0.0.1").arg("--port").arg(port.to_string());
if has_nvidia_gpu() { cmd.arg("-ngl").arg("999"); }
if info.ctx > 0 { cmd.arg("-c").arg(info.ctx.to_string()); }     // 8192 → avoids OOM
if info.mtp {
    if let (Some(d), true) = (&paths.drafter, gpu_vram_gb() >= info.min_vram_gb) {
        cmd.arg("-md").arg(d).arg("-ngld").arg("999").arg("-fa").arg("on")
           .arg("--spec-type").arg("draft-mtp").arg("--spec-draft-n-max").arg("4");
    } else {
        eprintln!("⚠ <{} GB VRAM or no drafter — running 31B without MTP (~42 tok/s).",
                  info.min_vram_gb);
    }
}
cmd.arg("--jinja");   // apply Gemma 4 chat template so tool/function calling works through eunice
```
For MTP, build with the model on one device; the binary path comes from `ensure_mtp_server()` rather
than `find_server_binary()` when `info.mtp` is true.

### 5. CLI / lifecycle (`src/main.rs`)

- `--download hf:gemma4:31b` → fetch both GGUFs **and** run `ensure_mtp_server()` (pre-warm the build).
- Add `--rebuild-gemma4-mtp` (force clone+build) and honor `EUNICE_GEMMA4_MTP_REF` to pin/override the
  branch/commit.
- `setup_local_model(alias)`: if `info.mtp`, call `ensure_mtp_server()` before `start_server`.

---

## Server invocation `start_server` produces

```bash
~/.eunice/build/llama.cpp-mtp/build/bin/llama-server \
  -m  ~/.eunice/models/google_gemma-4-31B-it-Q4_K_M.gguf \
  -md ~/.eunice/models/mtp-gemma-4-31B-it.gguf \
  -ngl 999 -ngld 999 -fa on -c 8192 -ctk q8_0 -ctv q8_0 \
  --spec-type draft-mtp --spec-draft-n-max 4 \
  --jinja --host 127.0.0.1 --port 18921
```
- `-c 8192` — **mandatory**; default context OOMs on 24 GB.
- `-ctk q8_0 -ctv q8_0` — **also required** for `llama-server`: f16 KV at 8192 OOMs by ~0.5 GB under
  the server's extra overhead. q8 KV fits with margin and keeps the full 8192 window (verified).
- `--spec-draft-n-max` — tune 2–6 (4 ≈ 2.3× here).
- old `--draft-max` is **removed**; use `--spec-draft-n-max`.

---

## Validation gate (`preflight_gemma4_mtp`) — runs first, no silent failures

A **single function aggregates every check and reports them all at once**, then proceeds only if all
*hard* requirements pass. It runs **before** any download or build, so failures cost <1 s, not a
19 GB download. It does not stop at the first failure — it collects them so the user can fix
everything in one pass.

### What it checks

| Class | Need | Check | On failure |
|---|---|---|---|
| hard | git | `git --version` | "Install git." |
| hard | cmake ≥ 3.18 | `cmake --version` (parse) | "Install cmake ≥ 3.18." |
| hard | C++ compiler | `c++ --version` / `cc` | "Install build-essential (Linux) / Xcode CLT (mac)." |
| hard | CUDA toolkit | `nvcc --version` (parse version) | "Install the CUDA toolkit — `nvcc` is required to build (the driver alone is not enough)." |
| hard | NVIDIA GPU | `nvidia-smi -L` succeeds | "MTP requires an NVIDIA GPU. Try `hf:gemma4:26b` (CPU/Metal) instead." |
| hard | disk space | free bytes on the FS holding `~/.eunice` ≥ **35 GB** | "Need ≥35 GB free in ~/.eunice; have N GB." |
| soft | VRAM ≥ 24 GB | `nvidia-smi --query-gpu=memory.total` | warn → run plain 31B (~42 tok/s), skip `-md` |
| soft | CUDA ≥ 12.x | parsed `nvcc` version | warn → build may be slow/unsupported on old toolkits |

**Disk budget** (the 35 GB figure): target GGUF ~19 GB + drafter ~0.5 GB + cloned source ~0.7 GB +
CUDA build artifacts ~6–8 GB + headroom. Steps that are already satisfied are subtracted (e.g. if the
binary is built and cached, only count the model download). Check the **specific filesystem** backing
`~/.eunice` (it may differ from `/`).

### Sketch (`src/local.rs`)

```rust
pub struct Check { pub name: &'static str, pub ok: bool, pub detail: String, pub hard: bool }

pub struct Preflight { pub checks: Vec<Check>, pub cuda_arch: Option<String>, pub run_mtp: bool }

impl Preflight {
    pub fn hard_failures(&self) -> Vec<&Check> {
        self.checks.iter().filter(|c| c.hard && !c.ok).collect()
    }
    pub fn ok(&self) -> bool { self.hard_failures().is_empty() }
}

/// Validate everything needed for gemma4:31b+MTP. Pure inspection — no downloads, no builds.
pub fn preflight_gemma4_mtp(info: &HfModelInfo) -> Preflight {
    let mut c = Vec::new();
    c.push(check_cmd("git",   &["--version"], true));
    c.push(check_cmake_min("3.18"));                       // hard
    c.push(check_cmd("c++",   &["--version"], true));
    let nvcc = check_cmd("nvcc", &["--version"], true);    // hard: toolkit, not just driver
    c.push(nvcc);
    let gpu  = check_cmd("nvidia-smi", &["-L"], true);
    c.push(gpu);

    // disk: only count what we still need to fetch/build
    let need_gb = required_disk_gb(info);                  // 35 fresh, less if cached
    let free_gb = free_space_gb(&eunice_dir());
    c.push(Check { name: "disk", ok: free_gb >= need_gb, hard: true,
                   detail: format!("{free_gb} GB free, need {need_gb} GB in {}", eunice_dir().display()) });

    // soft: VRAM gates MTP itself, not the build
    let vram = gpu_vram_gb();
    let run_mtp = vram >= info.min_vram_gb;
    c.push(Check { name: "vram", ok: run_mtp, hard: false,
                   detail: format!("{vram} GB (need {} for MTP; else plain 31B)", info.min_vram_gb) });

    Preflight { checks: c, cuda_arch: detect_cuda_arch(), run_mtp }
}
```
Call site in `setup_local_model` (before `ensure_models`/`ensure_mtp_server`):
```rust
if info.mtp {
    let pf = preflight_gemma4_mtp(&info);
    print_preflight(&pf);                 // always show the table
    if !pf.ok() {
        bail!("gemma4:31b prerequisites not met — fix the ✗ items above and retry.");
    }
    // pf.run_mtp / pf.cuda_arch flow into start_server / ensure_mtp_server
}
```

### Example output (failure aborts before any work)

```
$ eunice --model=gemma4:31b "hi"
Checking prerequisites for gemma4:31b + MTP…
  ✓ git              2.34.1
  ✓ cmake            3.22.1
  ✓ c++              g++ 12.3.0
  ✗ nvcc             not found — install the CUDA toolkit (driver alone is insufficient)
  ✓ nvidia-smi       NVIDIA GeForce RTX 4090
  ✗ disk             21 GB free, need 35 GB in /home/you/.eunice
  ⚠ vram             24 GB (ok for MTP)

error: gemma4:31b prerequisites not met — fix the ✗ items above and retry.
       (nothing was downloaded or built)
```
All-pass proceeds straight into download → build → run. `--download hf:gemma4:31b` runs the same gate
first so pre-warming fails fast too.

---

## Verification / acceptance criteria

1. Fresh machine: `eunice --model=gemma4:31b "hi"` downloads weights, builds the server, answers —
   no manual steps.
2. Second run starts in seconds (cache hit on binary + weights; `.gemma4-mtp.version` matches).
3. Throughput ≈ 80–100 tok/s; sanity-compare to ≈42 tok/s without the `-md` block.
4. Tool loop works: `eunice --model=gemma4:31b "list files then read README"` (validates `--jinja`).
5. Non-NVIDIA / <24 GB: graceful message + fallback (plain 31B or suggest `hf:gemma4:26b`), no OOM.
6. `--rebuild-gemma4-mtp` forces a clean rebuild; `EUNICE_GEMMA4_MTP_REF` pins a commit.
7. `cargo test`: add `resolve_hf_alias("gemma4:31b")` assertions (mtp, drafter, ctx==8192).

---

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| 10–15 min first-run build surprises users | Loud, clear "first run, building…" message + progress; pre-warm via `--download`. |
| CUDA toolkit absent (driver ≠ toolkit) | Preflight `nvcc`; precise install instructions; don't attempt build without it. |
| Wrong CUDA arch | Auto-detect via `compute_cap`; fall back to a broad arch list; allow override env. |
| WIP branch breaks/build fails | Pin a known-good commit (`EUNICE_GEMMA4_MTP_REF`); keep `build.log`; on failure fall back to E4B/26B with a clear note. |
| OOM at default ctx on 24 GB | Force `-c 8192`; VRAM-gate MTP flags; document `-ctk/-ctv q8_0`. |
| Concurrent builds | `flock` build lock in `~/.eunice/build`. |
| Disk (19 GB + source + build) | Preflight free-space check, warn early. |
| Occasional CUDA timeouts (PR-reported) | Detect server exit in agent loop; clear error + single retry. |

## Future (when PR #23398 merges to a release)

Replace `ensure_mtp_server()`'s clone+build with a plain release-binary download (like the existing
`install.sh` `gemma4-server` path, bumping `LLAMA_TAG`), keeping the same aliases, flags, and
`~/.eunice/` layout. The auto-build becomes the fallback for users on bleeding-edge models only.
```
