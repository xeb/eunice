# Image Understanding Example

This example demonstrates using Eunice's built-in `interpret_image` tool for multimodal image analysis.

## Files

- `instructions.md` - Prompt asking to analyze sample.jpg
- `sample.jpg` - Sample image to analyze
- `run.sh` - Script to run the example

## Usage

### With DMN Mode (includes all MCP tools)

```bash
cd examples/image_understanding
eunice --dmn --prompt instructions.md
```

### With --images Flag (standalone, no MCP)

```bash
cd examples/image_understanding
eunice --images --no-mcp "Describe the image at sample.jpg"
```

### Using run.sh

```bash
./run.sh
```

## How It Works

1. The model receives the prompt asking to analyze an image
2. It calls the `interpret_image` tool with:
   - `file_path`: Path to the image file
   - `prompt`: Analysis instructions
3. Eunice reads and base64-encodes the image
4. A multimodal API request is made to the configured provider
5. The analysis is returned as the tool result

## Requirements

- Eunice installed (`cargo install eunice`)
- A configured API key for a multimodal-capable provider:
  - `GEMINI_API_KEY` (recommended)
  - `OPENAI_API_KEY`
  - `ANTHROPIC_API_KEY`
