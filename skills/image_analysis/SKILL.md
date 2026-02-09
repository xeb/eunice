# Image Analysis Skill

## Description
Analyze images to extract metadata, dimensions, format info, text via OCR, and AI-powered descriptions using Gemini.

## Scripts

### analyze.py
A Python script for image analysis. Run with `uv run`:

```bash
# Get all info (metadata + OCR + AI description)
uv run ~/.eunice/skills/image_analysis/analyze.py image.png --all

# Just metadata
uv run ~/.eunice/skills/image_analysis/analyze.py photo.jpg --metadata

# Just OCR text extraction
uv run ~/.eunice/skills/image_analysis/analyze.py document.png --ocr

# AI description (requires GEMINI_API_KEY)
uv run ~/.eunice/skills/image_analysis/analyze.py photo.jpg --describe

# AI description with custom prompt
uv run ~/.eunice/skills/image_analysis/analyze.py photo.jpg --describe "What objects are visible?"

# Output as JSON
uv run ~/.eunice/skills/image_analysis/analyze.py screenshot.png --all --json
```

### Requirements
- `uv` (Python package manager)
- For OCR: Tesseract must be installed on the system
  - Linux: `apt install tesseract-ocr`
  - macOS: `brew install tesseract`
- For AI description: `GEMINI_API_KEY` environment variable

## Examples

### Screenshot Analysis
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py ~/Screenshots/error.png --describe "What error is shown?"
```

### Extract Text from Document
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py scanned_receipt.jpg --ocr
```

### Photo Description
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py vacation.jpg --describe "Describe the location and any landmarks"
```

### Diagram Understanding
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py architecture.png --describe "Explain this system architecture diagram"
```

### UI/UX Review
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py mockup.png --describe "Critique this UI design"
```

### Code Screenshot
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py code.png --describe "What does this code do? Are there any bugs?"
```

### Get Image Dimensions
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py banner.png --metadata --json | jq '.metadata.size'
```

### Check EXIF Data (Camera Info)
```bash
uv run ~/.eunice/skills/image_analysis/analyze.py photo.jpg --metadata
```
