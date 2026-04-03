# Gemma 4 Local Inference in Eunice — Research & Design

## Decisions

- **Model ID**: `hf:gemma4:e4b` / `hf:gemma4:26b` — `hf:` prefix signals HuggingFace download path, plain `gemma4:e4b` still routes to Ollama
- **Default quant**: Q4_K_M (~4.5 GB for E4B, ~16 GB for 26B)
- **Inference backend**: `gemma4-server` (llama-server from llama.cpp) as subprocess
- **Server lifecycle**: Dies when eunice exits. Show "Starting gemma4-server..." on startup
- **GPU detection**: Auto-detect NVIDIA GPU → download CUDA build; otherwise CPU/Metal
- **Distribution**: gemma4-server bundled via install.sh on longrunningagents.com (same as eunice)
- **Scope**: Gemma 4 only (not a general local model runner)
- **Weight source**: HuggingFace only (the only practical source for GGUF weights; Kaggle exists but requires more auth)

---

## Model Overview

| Model | HuggingFace Repo | Architecture | Total Params | Active Params | BF16 Size | Context |
|-------|-----------------|-------------|-------------|--------------|-----------|---------|
| **E4B** | `google/gemma-4-E4B-it` | Dense + Per-Layer Embeddings | ~8B | 4.5B | ~48 GB | 128K |
| **26B-A4B** | `google/gemma-4-26B-A4B-it` | MoE (128 experts, top-8) | ~26.5B | 3.8B | ~205 GB | 256K |

Both are multimodal (text + image; E4B also supports audio). License: Apache 2.0.

### Why E4B is 48 GB Despite Being "4B"

Gemma 4 E4B uses **Per-Layer Embeddings (PLE)** — each of its 42 decoder layers has its own 262K-vocab embedding table, inflating disk size far beyond what the effective parameter count suggests.

### GGUF Quantized Versions (Recommended for Local Use)

Available from `unsloth/gemma-4-E4B-it-GGUF` and `unsloth/gemma-4-26B-A4B-it-GGUF`:

| Quant | E4B Size (approx) | 26B Size (approx) | Quality |
|-------|-------------------|-------------------|---------|
| Q8_0 | ~8 GB | ~28 GB | Near-lossless |
| Q5_K_M | ~5.5 GB | ~19 GB | Good |
| **Q4_K_M** | **~4.5 GB** | **~16 GB** | **Default** |
| Q3_K_M | ~3.5 GB | ~12 GB | Degraded |
| IQ4_XS | ~4 GB | ~14 GB | Good (imatrix) |

---

## Downloading Weights from Rust

### `hf-hub` crate (official HuggingFace Rust SDK)

```toml
[dependencies]
hf-hub = { version = "0.5", features = ["tokio"] }
```

```rust
use hf_hub::api::tokio::Api;

let api = Api::new().unwrap();
let repo = api.repo(hf_hub::Repo::model("unsloth/gemma-4-E4B-it-GGUF".to_string()));
let model_path = repo.get("gemma-4-E4B-it-Q4_K_M.gguf").await.unwrap();
```

Key features:
- **Resumable downloads** with HTTP Range headers
- **Progress reporting** via `download_with_progress()` trait
- **Cache-compatible** with Python `huggingface_hub` (`~/.cache/huggingface/hub/`)
- **Cross-platform file locking** to prevent concurrent downloads
- **Auth support** via `HF_TOKEN` env var or `~/.cache/huggingface/token`
- Handles multi-GB files with streaming (not buffered in memory)

### Why HuggingFace Only

| Source | GGUF? | Direct HTTP? | Auth Required? |
|--------|-------|-------------|----------------|
| HuggingFace (community quants like unsloth) | Yes | Yes | Often no (ungated) |
| HuggingFace (official Google) | Yes | Yes | Yes (gated) |
| Kaggle | Yes | Yes (API) | Yes (API key + license acceptance) |
| Ollama registry | Internal format | Proprietary protocol | No |

Community GGUF repos (unsloth, bartowski) on HuggingFace are the path of least resistance — often ungated, plain HTTP download.

---

## Inference Backend: Why llama-server

### Rejected Options

| Option | Why Rejected |
|--------|-------------|
| **vLLM** | CUDA-only (no Apple Silicon), 30s-5min startup, heavy Python/PyTorch dependency (~2-5 GB), CPU mode unusably slow |
| **gemma.cpp** | Google's lightweight C++ runtime — **never got Gemma 4 support**, appears abandoned after Gemma 2 |
| **candle (Rust)** | Gemma 4 support unclear/incomplete, no built-in server, would need to implement model architecture |
| **Embedded mistral.rs** | No pre-built binaries, would massively increase compile time and binary size |
| **Embedded llama-cpp-2** | C++ FFI complexity, CMake build dependency, binary size bloat |
| **MLX direct** | Mac-only, `mlx-rs` Rust bindings still RC. Ollama on Mac already uses MLX backend |

### Winner: `gemma4-server` (llama-server from llama.cpp)

- Pre-built binaries for **all platforms** (Linux x64/ARM64, macOS x64/ARM64, Windows x64/ARM64)
- **Metal acceleration built-in** on Apple Silicon — no special flags
- GGUF-native — the primary format, not an afterthought
- **OpenAI-compatible API** with tool/function calling support
- Battle-tested, massive community, daily releases
- Small binary (~5-15 MB for llama-server, ~30-40 MB archive)
- Fast startup (seconds, not minutes)
- CPU, CUDA, Metal, and Vulkan support

### Pre-built Binary Downloads

Release URL pattern: `https://github.com/ggerganov/llama.cpp/releases/download/{tag}/llama-{tag}-bin-{platform}.tar.gz`

| Platform | Asset | Archive Size |
|----------|-------|-------------|
| macOS ARM64 (Apple Silicon) | `llama-{tag}-bin-macos-arm64.tar.gz` | ~38 MB |
| macOS x64 | `llama-{tag}-bin-macos-x64.tar.gz` | ~99 MB |
| Linux x64 | `llama-{tag}-bin-ubuntu-x64.tar.gz` | ~30 MB |
| Linux ARM64 | `llama-{tag}-bin-ubuntu-arm64.tar.gz` | ~27 MB |
| Windows x64 | `llama-{tag}-bin-win-cpu-x64.zip` | ~37 MB |
| Windows ARM64 | `llama-{tag}-bin-win-cpu-arm64.zip` | ~31 MB |
| Linux x64 (CUDA) | `llama-{tag}-bin-ubuntu-cuda-x64.tar.gz` | ~150+ MB |
| Windows x64 (CUDA 12.4) | `llama-{tag}-bin-win-cuda-12.4-x64.zip` | ~238 MB |

---

## Architecture

### Flow

```
User: eunice --model hf:gemma4:e4b "explain this code"

Eunice:
  1. Detect "hf:" prefix → HuggingFace/local mode (not Ollama)
  2. Resolve alias: gemma4:e4b → unsloth/gemma-4-E4B-it-GGUF / Q4_K_M
  3. Check ~/.eunice/models/ for cached GGUF weights
     → If missing: download via hf-hub with progress bar
  4. Check ~/.eunice/bin/ for gemma4-server
     → If missing: error with install instructions
  5. Print "Starting gemma4-server..."
  6. Spawn gemma4-server as subprocess:
     gemma4-server -m ~/.eunice/models/gemma-4-E4B-it-Q4_K_M.gguf \
       --port 18921 --host 127.0.0.1
  7. Poll http://127.0.0.1:18921/health until ready
  8. Route via existing OpenAI-compatible client to 127.0.0.1:18921/v1/
  9. On exit: kill gemma4-server subprocess
```

### NVIDIA GPU Auto-Detection

```rust
fn has_nvidia_gpu() -> bool {
    // Check for nvidia-smi
    std::process::Command::new("nvidia-smi")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

If NVIDIA GPU detected → download CUDA build of gemma4-server.
Otherwise → CPU build (Linux/Windows) or Metal build (macOS ARM64).

### Model ID Format

```
Default (Ollama):     eunice --model gemma4:e4b        → routes to Ollama as today
HuggingFace local:    eunice --model hf:gemma4:e4b     → downloads weights, runs gemma4-server
With quantization:    eunice --model hf:gemma4:e4b-q8  → Q8_0 quantization
```

The `hf:` prefix preserves backward compatibility — `gemma4:e4b` without `hf:` still routes to Ollama.

### Model Aliases

```
hf:gemma4:e4b      → unsloth/gemma-4-E4B-it-GGUF (Q4_K_M, ~4.5 GB)
hf:gemma4:e4b-q8   → unsloth/gemma-4-E4B-it-GGUF (Q8_0, ~8 GB)
hf:gemma4:e4b-q5   → unsloth/gemma-4-E4B-it-GGUF (Q5_K_M, ~5.5 GB)
hf:gemma4:26b      → unsloth/gemma-4-26B-A4B-it-GGUF (Q4_K_M, ~16 GB)
hf:gemma4:26b-q8   → unsloth/gemma-4-26B-A4B-it-GGUF (Q8_0, ~28 GB)
```

### Apple Silicon

Metal acceleration comes for free with the macOS ARM64 gemma4-server binary. No special detection or configuration needed — llama-server auto-detects Metal on macOS. Apple Silicon's unified memory is especially beneficial for the 26B MoE model.

---

## CLI Design

### New Commands

```bash
# Use local model (auto-downloads weights if needed, requires gemma4-server installed)
eunice --model hf:gemma4:e4b "explain this code"
eunice --model hf:gemma4:e4b --chat
eunice --model hf:gemma4:26b --chat

# Pre-download model weights
eunice --download hf:gemma4:e4b
eunice --download hf:gemma4:e4b-q8

# Manage local models
eunice --local-models                     # List downloaded models + sizes
eunice --remove-model gemma4:e4b          # Delete cached weights

# Start as standalone server (for other tools to use)
eunice --serve hf:gemma4:e4b              # Start gemma4-server on default port
eunice --serve hf:gemma4:e4b --port 8080  # Custom port
```

### User Experience

```
$ eunice --model hf:gemma4:e4b "what is 2+2?"

Downloading gemma-4-E4B-it-Q4_K_M.gguf (4.5 GB)...
[████████████████████████████████████] 100% (4.5 GB / 4.5 GB)

Starting gemma4-server...
Ready.

2 + 2 = 4.
```

On subsequent runs (weights cached):
```
$ eunice --model hf:gemma4:e4b "what is 2+2?"

Starting gemma4-server...
Ready.

2 + 2 = 4.
```

---

## Storage Layout

```
~/.eunice/
├── bin/
│   └── gemma4-server                     # llama-server binary (installed via install.sh)
├── models/
│   ├── gemma-4-E4B-it-Q4_K_M.gguf       # Downloaded model weights
│   ├── gemma-4-26B-A4B-it-Q4_K_M.gguf
│   └── models.json                       # Metadata (download date, size, source URL, quant)
└── skills/                               # Existing skills directory
```

---

## Distribution via install.sh

The existing `install.sh` at `~/gal/p/longrunningagents.com/install.sh` installs eunice and auntie via `cargo install --git`. We add a section to download the platform-appropriate gemma4-server binary.

### Changes to install.sh

After the existing eunice/auntie cargo install sections, add:

```bash
echo ""
echo "Installing gemma4-server (local inference engine)..."
echo ""

# Pin to a known-good llama.cpp release
LLAMA_TAG="b8642"

# Detect platform for gemma4-server binary
case "$OS-$ARCH" in
    Linux-x86_64|Linux-amd64)
        if command -v nvidia-smi &> /dev/null && nvidia-smi &> /dev/null; then
            LLAMA_ASSET="llama-${LLAMA_TAG}-bin-ubuntu-cuda-x64.tar.gz"
        else
            LLAMA_ASSET="llama-${LLAMA_TAG}-bin-ubuntu-x64.tar.gz"
        fi
        ;;
    Linux-aarch64|Linux-arm64)
        LLAMA_ASSET="llama-${LLAMA_TAG}-bin-ubuntu-arm64.tar.gz"
        ;;
    Darwin-arm64|Darwin-aarch64)
        LLAMA_ASSET="llama-${LLAMA_TAG}-bin-macos-arm64.tar.gz"
        ;;
    Darwin-x86_64)
        LLAMA_ASSET="llama-${LLAMA_TAG}-bin-macos-x64.tar.gz"
        ;;
    *)
        echo -e "${YELLOW}⚠${NC} No gemma4-server binary available for $OS-$ARCH"
        echo "  Local Gemma 4 inference will not be available"
        LLAMA_ASSET=""
        ;;
esac

if [ -n "$LLAMA_ASSET" ]; then
    LLAMA_URL="https://github.com/ggerganov/llama.cpp/releases/download/${LLAMA_TAG}/${LLAMA_ASSET}"
    EUNICE_BIN="$HOME/.eunice/bin"
    mkdir -p "$EUNICE_BIN"

    TMPDIR=$(mktemp -d)
    echo "Downloading $LLAMA_ASSET..."
    if curl -L -o "$TMPDIR/llama.tar.gz" "$LLAMA_URL"; then
        tar -xzf "$TMPDIR/llama.tar.gz" -C "$TMPDIR"
        # llama-server is inside the extracted archive
        find "$TMPDIR" -name "llama-server" -type f -exec cp {} "$EUNICE_BIN/gemma4-server" \;
        chmod +x "$EUNICE_BIN/gemma4-server"
        rm -rf "$TMPDIR"
        echo -e "${GREEN}✓${NC} gemma4-server installed to $EUNICE_BIN/gemma4-server"
    else
        echo -e "${YELLOW}⚠${NC} Failed to download gemma4-server"
        echo "  You can install it later or use Ollama for local inference"
        rm -rf "$TMPDIR"
    fi
fi
```

### Updated install.sh output

```
==========================================
  Long Running Agents Installer
  (eunice + auntie + gemma4-server)
==========================================

...

Installed:
  eunice         - agentic CLI runner
  auntie         - autonomous agent framework
  gemma4-server  - local Gemma 4 inference engine

Quick start:
  eunice --model hf:gemma4:e4b --chat     # Local Gemma 4 (downloads weights on first use)
  eunice --model gemma4:e4b --chat         # Via Ollama (if available)
```

---

## Implementation Plan

### New Files

1. **`src/local.rs`** (~300-500 lines):
   - `resolve_hf_model(alias) -> (repo, filename, size)` — alias to HuggingFace coordinates
   - `download_model(alias) -> PathBuf` — download GGUF via hf-hub with progress bar
   - `find_server_binary() -> Option<PathBuf>` — look for gemma4-server in ~/.eunice/bin/
   - `start_server(model_path, port) -> Child` — spawn subprocess, print "Starting gemma4-server..."
   - `wait_for_ready(port)` — poll /health endpoint with timeout
   - `list_local_models() -> Vec<ModelInfo>` — scan ~/.eunice/models/
   - `remove_model(alias)` — delete cached weights

### Changed Files

2. **`src/models.rs`** — Add `Provider::Local` variant + icon
3. **`src/provider.rs`** — Detect `hf:` prefix, route to Local provider, tool support for gemma4
4. **`src/client.rs`** — Handle `Provider::Local` auth (none)
5. **`src/main.rs`** — Add `--download`, `--local-models`, `--remove-model`, `--serve` CLI args

### New Dependencies

```toml
[dependencies]
hf-hub = { version = "0.5", features = ["tokio"] }
```

---

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| llama.cpp releases break Gemma 4 | Pin to known-good release tag in install.sh |
| Large downloads on metered connections | Show size before downloading, require `--download` or show confirmation prompt |
| Port conflicts | Use high random port (18921+), make configurable |
| gemma4-server crashes mid-session | Detect process exit, show clear error |
| Disk space | Check available space before download, warn if insufficient |
| gemma4-server not installed | Clear error: "gemma4-server not found. Run install.sh or visit longrunningagents.com" |
| HuggingFace rate limits | hf-hub handles retries; cached downloads avoid repeat fetches |
