# PDF Analysis Task

Analyze the PDF at `sample.pdf` in this directory.

Use these command-line tools (install with `apt install poppler-utils` if needed):

```bash
# Get PDF metadata
pdfinfo sample.pdf

# Extract text from PDF
pdftotext sample.pdf -

# Get page count
pdfinfo sample.pdf | grep Pages
```

Describe what you find in the PDF, including any text content and metadata.
