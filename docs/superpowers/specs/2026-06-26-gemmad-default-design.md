# Design: `--gemmad` daemon mode as global default

**Date:** 2026-06-26
**Status:** Approved (pre-implementation)
**Component:** eunice provider/model selection

## Problem

`eunice --gemma` builds and starts a local **gemma-4-31B + MTP** server on port
18921. On this machine a separate daemon, **`gemmad`**, is already running and
serving **gemma-4-12b** on port 18082, holding the RTX 4090's VRAM. So `--gemma`
OOMs while loading the 31B weights:

```
ggml_backend_cuda_buffer_type_alloc_buffer: allocating 18675.55 MiB on device 0: cudaMalloc failed: out of memory
gemma4-mtp-server exited before becoming ready (exit status: 1).
```

The two servers are unrelated:

| | `--gemma` (current) | `gemmad` (already running) |
|---|---|---|
| Model | gemma-4-31B + MTP drafter | gemma-4-12b |
| Port | 18921 (eunice builds & starts) | 18082 (already running) |
| Auth | none | Bearer token (`~/.config/gemmad/keys.toml`) |
| API | OpenAI-compatible (llama-server) | OpenAI-compatible |
| VRAM | needs ~24 GB | already resident |

We want eunice to **detect the running gemmad and route to it** instead of
building its own server.

## Goals

- Add `--gemmad` to use the running daemon; make a reachable daemon the
  **global default** for plain `eunice`.
- Never silently start a server in this path — just talk to the existing daemon
  over its OpenAI-compatible API.
- Add Bearer-token auth (the daemon requires it; eunice's `Local` provider
  currently sends none).
- Leave `--gemma` (the 31B MTP build path) unchanged.

## Non-goals

- No changes to the 31B MTP build/download/preflight logic.
- No remote/multi-host daemon discovery beyond `GEMMAD_HOST`/`GEMMAD_PORT`.
- No new model-management UI.

## Decisions (locked)

1. **Global default scope.** A reachable gemmad becomes the auto-selected model
   for plain `eunice` (no model/provider flag), overriding `get_smart_default_model()`.
   `--gemmad` forces it (errors if unreachable); `--no-gemmad` opts out and uses
   the cloud smart-default.
2. **Token source.** `$GEMMAD_API_KEY` → else `~/.config/gemmad/keys.toml`
   `[keys].dev` (fall back to first key) → else error.
3. **`--gemma` unchanged.** Stays the explicit "build & start the 31B MTP server"
   path. It's the escape hatch for actually wanting 31B (stop gemmad first to
   free VRAM).

## Behavior / flag semantics

Two new flags in `src/main.rs`:

- `--gemmad` — force the running gemmad daemon; **error if not reachable**.
- `--no-gemmad` — disable auto-detection; fall back to the cloud smart-default.

Model-selection precedence (replaces the current `if args.gemma { … } else { … }`
block around `src/main.rs:404`):

| Priority | Condition | Result |
|---|---|---|
| 1 | `--gemma` | unchanged → build/start local 31B MTP server |
| 2 | `--gemmad` | gemmad reachable? use it; else **error** |
| 3 | `--model X` given | use X (explicit always wins) |
| 4 | nothing, gemmad reachable, not `--no-gemmad` | **auto-use gemmad** |
| 5 | otherwise | `get_smart_default_model()` (today's behavior) |

Mutually-exclusive guards with clear errors:

- `--gemma` + `--gemmad`
- `--gemmad` + `--no-gemmad`
- `--gemmad` + `--model X` where X ≠ the gemmad model id
  (mirrors the existing `--gemma` + `--model` check)

When the daemon is auto-selected (priority 4) or forced (priority 2), eunice
prints a one-line notice so it is never silent about overriding the default:

```
Using local gemmad (gemma-4-12b) at 127.0.0.1:18082
```

## Detection

Quick `GET http://{host}:{port}/livez` (no auth) with a ~400 ms timeout. HTTP 200
⇒ available. Connection-refused returns immediately on localhost, so plain
`eunice` pays negligible latency when the daemon is down. Host/port overridable
via `GEMMAD_HOST` / `GEMMAD_PORT` (the same env vars gemmad itself uses);
defaults `127.0.0.1` / `18082`.

## Provider routing & auth

- New `Provider::Gemmad` variant in `src/models.rs`.
- `src/provider.rs::detect_provider()` gains an early, explicit branch: when the
  model equals the gemmad model id (default `gemma-4-12b`, overridable via
  `GEMMAD_MODEL_ID`), return:

  ```rust
  ProviderInfo {
      provider: Provider::Gemmad,
      base_url: "http://127.0.0.1:18082/v1/",   // from gemmad::base_url()
      api_key: <resolved token>,                 // from gemmad::resolve_token()
      resolved_model: "gemma-4-12b (local gemmad)",
      use_native_gemini_api: false,
      azure_api_version: None,
  }
  ```

- `src/client.rs::add_auth()` gains a `Provider::Gemmad => Bearer <token>` branch.
  `Local` stays no-auth. Everything else — OpenAI-compatible chat completions,
  SSE streaming, `{base_url}chat/completions` URL building — reuses the existing
  default path.
- Because the provider is **not** `Provider::Local`, the "start local server"
  block in `src/main.rs` (~line 425) is skipped automatically: no preflight, no
  download, no build, no OOM.
- Usage/cost: treat Gemmad like Local (free/local) wherever per-provider pricing
  is computed (`src/usage.rs`), so token accounting doesn't choke on an unpriced
  model.

The model string sent in the request JSON is the gemmad model id (`gemma-4-12b`),
matching gemmad's advertised `GEMMAD_MODEL_ID`. As a side effect,
`--model gemma-4-12b` also routes to the daemon.

## New module: `src/gemmad.rs`

Isolates all daemon-specific concerns so the rest of the code stays clean and the
logic is unit-testable:

- Constants: default host (`127.0.0.1`), port (`18082`), model id (`gemma-4-12b`),
  keys-file path (`~/.config/gemmad/keys.toml`).
- `fn host() -> String` / `fn port() -> u16` / `fn model_id() -> String` — read
  env overrides with defaults.
- `fn base_url() -> String` → `http://{host}:{port}/v1/`.
- `async fn is_available() -> bool` — the `/livez` probe with timeout.
- `fn resolve_token() -> Result<String>` — env → keys.toml `[keys].dev`/first →
  error with actionable message.
- `enum ModelChoice { Explicit(String), Gemmad, Gemma31b, SmartDefault }` and a
  pure, network-free `fn decide_model(gemma: bool, gemmad: bool, no_gemmad: bool,
  model: Option<&str>, gemmad_up: bool) -> Result<ModelChoice>` implementing the
  precedence table above. `main.rs` resolves `SmartDefault` via the existing
  `get_smart_default_model()` and `Gemmad`/`Gemma31b` to their model strings. This
  keeps the live `/livez` check out of the decision logic so it can be tested.

`keys.toml` parsing uses the `toml` crate (add to `Cargo.toml` if not already a
dependency; `serde` is already present). Read the `[keys]` table, prefer `dev`,
else take the first entry.

## Open risk to verify during implementation

gemmad is confirmed OpenAI-compatible for chat + vision + SSE streaming, but
**function-calling / `tools` support is unconfirmed**. eunice's agent loop sends
the 4 built-in tools (Bash/Read/Write/Skill). Plan:

1. Early in implementation, send a real chat request *with* `tools` to the live
   daemon and observe.
2. If gemmad ignores unknown `tools`: chat works (covers the `--gemma --chat` use
   case); leave tools enabled.
3. If gemmad rejects requests carrying `tools`: gate tools off for
   `Provider::Gemmad` using the same mechanism Ollama uses
   (`provider::supports_tools()`).

Surface the actual finding rather than assuming.

## Tests

- `gemmad::resolve_token` precedence: env set wins; env unset + keys.toml `dev`;
  keys.toml without `dev` (first key); neither → error.
- `detect_provider(gemmad model id)` → `Provider::Gemmad` with correct base_url,
  api_key, resolved_model.
- `client::add_auth` emits `Authorization: Bearer …` for `Provider::Gemmad` and
  nothing for `Local`.
- `gemmad::decide_model` precedence table across the flag combinations and the
  `gemmad_up` boolean, including the mutually-exclusive error cases.

## Release housekeeping (per CLAUDE.md)

- `cargo test` green.
- Update LOC + binary size in `README.md`.
- Document `--gemmad` / `--no-gemmad` in help/README.
- Version bump + git hash handled by existing `build.rs` flow at release time.
