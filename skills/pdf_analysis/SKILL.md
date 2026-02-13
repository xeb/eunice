# PDF Analysis Skill

## Description
Analyze PDF files to extract text, metadata, page count, and images. Supports OCR for scanned documents.

## Scripts

### analyze.py
A Python script for PDF analysis. Run with `uv run`:

```bash
# Get all info (metadata + text + page count)
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --all

# Just metadata
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --metadata

# Extract text content
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --text

# Extract text with OCR (for scanned PDFs)
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --ocr

# List images in the PDF
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --images

# Extract images to a directory
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --extract-images ./output

# Output as JSON
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --all --json
```

### Requirements
- `uv` (Python package manager)
- For OCR: Tesseract must be installed (`apt install tesseract-ocr`)
- For image extraction: poppler-utils (`apt install poppler-utils`)

## Examples

### Analyze a Report
```bash
uv run ~/.eunice/skills/pdf_analysis/analyze.py report.pdf --all
```

### Extract Text from Scanned Document
```bash
uv run ~/.eunice/skills/pdf_analysis/analyze.py scanned.pdf --ocr
```

### Get Page Count
```bash
uv run ~/.eunice/skills/pdf_analysis/analyze.py book.pdf --metadata --json | jq '.pages'
```

### Check if PDF is Searchable
```bash
# If --text returns nothing but --ocr returns text, it's a scanned PDF
uv run ~/.eunice/skills/pdf_analysis/analyze.py document.pdf --text
```
