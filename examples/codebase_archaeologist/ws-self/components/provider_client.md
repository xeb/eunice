# Component: Provider & Client Layer
**Path:** `src/provider.rs`, `src/client.rs`, `src/models.rs`
**Last Analyzed:** 2025-11-25 19:45
**Primary Author(s):** (Unknown - no git history)

## Purpose
This layer abstracts the differences between various LLM providers (OpenAI, Anthropic, Gemini, Ollama), providing a unified interface for the rest of the application. It handles authentication, model resolution (including aliasing), and HTTP communication.

## Structure
- `src/provider.rs`: Logic for detecting providers from model names, checking availability (Ollama), and resolving model aliases. Contains "Smart Default" logic.
- `src/client.rs`: A `reqwest`-based HTTP client. Mostly speaks OpenAI-compatible JSON, but has special branches for Native Gemini API.
- `src/models.rs`: Shared data structures (`ProviderInfo`, `ChatCompletionRequest`, `Message`) and API-specific types (Gemini Native, MCP config).

## Key Patterns
- **Unified Provider Abstraction**: The `Provider` enum and `ProviderInfo` struct normalize connection details.
- **Dual-Mode Gemini Support**: Supports both Google's OpenAI-compatible endpoint and their Native API (required for "Gemini 3" features like `thoughtSignature`).
- **Future-Aware Aliasing**: `resolve_anthropic_alias` maps short names (e.g., "sonnet") to specific versioned identifiers (e.g., "claude-sonnet-4-20250514").
- **Smart Defaults**: `get_smart_default_model` checks environment variables to pick the best available provider/model automatically.
- **Ollama Integration**: dynamic checking of local Ollama models via `/api/tags`.

## Dependencies
**Internal:** None (foundation layer).
**External:** `reqwest` (HTTP), `serde`/`serde_json` (Serialization), `anyhow` (Error handling).

## Concerns
- **Hardcoded Models**: The list of models and aliases is hardcoded. While currently up-to-date (2025), it requires manual updates.
- **Retry Logic**: `client.rs` implements a simple loop for retries but complex backoff/jitter isn't immediately obvious in the snippet.
- **Maintenance**: "Gemini 3" support implies bleeding-edge features that might change.

## Notes
- The codebase explicitly references models from late 2025 (GPT-5.1, Claude 4.5), confirming the project's timeframe.
- The `thoughtSignature` field in Gemini models suggests usage of Chain-of-Thought or reasoning capabilities exposed via API.
