
## 5. Multimodal Context vs Vision-as-a-Tool

*   **UNISS (Baseline):** Supports **native multimodal context**. It uses `GeminiPart::InlineData` to embed image data directly into the conversation history. This allows the model to "see" the image across multiple conversation turns.
*   **Eunice:** Implements vision as a **tool** (`interpret_image`). When an image is analyzed, `eunice` makes a separate, transient API call to describe the image, and only the *text description* is saved to the conversation history. The model loses access to the actual visual data after the tool call completes.

## 6. Google Internal Integrations

*   **UNISS (Baseline):** Contains hardcoded support for Google-internal MCP servers (e.g., `codemind`, `engage`) and handles internal build environment variables like `BUILD_WORKING_DIRECTORY`.
*   **Eunice:** Designed as a general-purpose open-source tool. It lacks these internal integrations but provides a generic `research` mode and standard MCP support.

## 7. Build & Distribution

*   **UNISS (Baseline):** Uses **Blaze** (Google's internal build system) and builds from source locally, detecting OS/Arch.
*   **Eunice:** Uses **Cargo** (Rust standard) and includes an update mechanism that checks a remote version file (`version.txt`) to prompt for updates.

## 8. Data Structures & Provider Specifics

*   **UNISS (Baseline):** Strongly typed to Gemini's API structure (`GeminiContent`, `GeminiPart`). It supports specific fields like `functionResponse` (JSON) and `thought_signature` (for Gemini 3+ thinking models).
*   **Eunice:** Uses a generic `Message` enum (similar to OpenAI's chat format) to support multiple providers (OpenAI, Anthropic, Gemini, Ollama). This abstraction may limit access to provider-specific features like native function response types or specific caching controls.
