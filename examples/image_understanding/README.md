# Image Understanding Example

This example demonstrates using Eunice's image_analysis skill for multimodal image analysis.

## Files

- `instructions.md` - Prompt asking to analyze sample.jpg
- `sample.jpg` - Sample image to analyze
- `run.sh` - Script to run the example

## Usage

```bash
cd examples/image_understanding
./run.sh
```

Or directly:

```bash
eunice --prompt instructions.md "Analyze the image"
```

## How It Works

1. The agent receives the prompt asking to analyze an image
2. It uses the Skill tool to find the image_analysis skill
3. It runs the analyze.py script via Bash to analyze the image
4. The analysis is returned

## Requirements

- Eunice installed
- `uv` (Python package manager)
- For AI description: `GEMINI_API_KEY` environment variable
- For OCR: Tesseract (`apt install tesseract-ocr` or `brew install tesseract`)
