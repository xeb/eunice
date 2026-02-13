# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "pymupdf>=1.24.0",
#     "pytesseract>=0.3.10",
#     "pillow>=10.0.0",
# ]
# ///
"""PDF Analysis Tool - Extract text, metadata, and images from PDFs."""

import argparse
import json
import sys
import subprocess
from pathlib import Path

try:
    import fitz  # PyMuPDF
except ImportError:
    print("Error: PyMuPDF not installed. Run with: uv run this_script.py", file=sys.stderr)
    sys.exit(1)


def get_metadata(doc: fitz.Document, pdf_path: Path) -> dict:
    """Extract PDF metadata."""
    meta = doc.metadata
    return {
        "file": str(pdf_path),
        "pages": doc.page_count,
        "title": meta.get("title", ""),
        "author": meta.get("author", ""),
        "subject": meta.get("subject", ""),
        "creator": meta.get("creator", ""),
        "producer": meta.get("producer", ""),
        "creation_date": meta.get("creationDate", ""),
        "mod_date": meta.get("modDate", ""),
        "encrypted": doc.is_encrypted,
        "file_size_bytes": pdf_path.stat().st_size,
    }


def extract_text(doc: fitz.Document) -> str:
    """Extract text from all pages."""
    text_parts = []
    for page_num in range(doc.page_count):
        page = doc[page_num]
        text = page.get_text()
        if text.strip():
            text_parts.append(f"--- Page {page_num + 1} ---\n{text}")
    return "\n\n".join(text_parts) if text_parts else "(No text found)"


def extract_text_ocr(pdf_path: Path) -> str:
    """Extract text using OCR (for scanned PDFs)."""
    try:
        import pytesseract
        from PIL import Image
        import io
    except ImportError:
        return "(OCR requires pytesseract and pillow)"

    doc = fitz.open(pdf_path)
    text_parts = []

    for page_num in range(doc.page_count):
        page = doc[page_num]
        # Render page to image at 300 DPI
        mat = fitz.Matrix(300/72, 300/72)
        pix = page.get_pixmap(matrix=mat)
        img = Image.open(io.BytesIO(pix.tobytes("png")))

        try:
            text = pytesseract.image_to_string(img)
            if text.strip():
                text_parts.append(f"--- Page {page_num + 1} (OCR) ---\n{text}")
        except Exception as e:
            text_parts.append(f"--- Page {page_num + 1} (OCR failed: {e}) ---")

    doc.close()
    return "\n\n".join(text_parts) if text_parts else "(No text detected via OCR)"


def list_images(doc: fitz.Document) -> list:
    """List all images in the PDF."""
    images = []
    for page_num in range(doc.page_count):
        page = doc[page_num]
        image_list = page.get_images()
        for img_idx, img in enumerate(image_list):
            xref = img[0]
            base_image = doc.extract_image(xref)
            images.append({
                "page": page_num + 1,
                "index": img_idx + 1,
                "width": base_image["width"],
                "height": base_image["height"],
                "format": base_image["ext"],
                "size_bytes": len(base_image["image"]),
            })
    return images


def extract_images(doc: fitz.Document, output_dir: Path, pdf_path: Path) -> list:
    """Extract all images to a directory."""
    output_dir.mkdir(parents=True, exist_ok=True)
    extracted = []

    for page_num in range(doc.page_count):
        page = doc[page_num]
        image_list = page.get_images()
        for img_idx, img in enumerate(image_list):
            xref = img[0]
            base_image = doc.extract_image(xref)
            image_bytes = base_image["image"]
            image_ext = base_image["ext"]

            filename = f"{pdf_path.stem}_p{page_num+1}_img{img_idx+1}.{image_ext}"
            output_path = output_dir / filename
            output_path.write_bytes(image_bytes)
            extracted.append(str(output_path))

    return extracted


def main():
    parser = argparse.ArgumentParser(description="Analyze PDF files")
    parser.add_argument("pdf_path", help="Path to the PDF file")
    parser.add_argument("--all", action="store_true", help="Show all info (metadata + text)")
    parser.add_argument("--metadata", action="store_true", help="Show metadata")
    parser.add_argument("--text", action="store_true", help="Extract text")
    parser.add_argument("--ocr", action="store_true", help="Extract text using OCR")
    parser.add_argument("--images", action="store_true", help="List images in the PDF")
    parser.add_argument("--extract-images", metavar="DIR", help="Extract images to directory")
    parser.add_argument("--json", action="store_true", help="Output as JSON")

    args = parser.parse_args()

    pdf_path = Path(args.pdf_path)
    if not pdf_path.exists():
        print(f"Error: File not found: {pdf_path}", file=sys.stderr)
        sys.exit(1)

    # Default to --all if no specific option given
    if not any([args.metadata, args.text, args.ocr, args.images, args.extract_images]):
        args.all = True

    try:
        doc = fitz.open(pdf_path)
    except Exception as e:
        print(f"Error opening PDF: {e}", file=sys.stderr)
        sys.exit(1)

    result = {}
    output_lines = []

    # Metadata
    if args.metadata or args.all:
        meta = get_metadata(doc, pdf_path)
        result["metadata"] = meta
        if not args.json:
            output_lines.append("=== PDF Metadata ===")
            output_lines.append(f"File: {meta['file']}")
            output_lines.append(f"Pages: {meta['pages']}")
            output_lines.append(f"Title: {meta['title'] or '(none)'}")
            output_lines.append(f"Author: {meta['author'] or '(none)'}")
            output_lines.append(f"Creator: {meta['creator'] or '(none)'}")
            output_lines.append(f"Created: {meta['creation_date'] or '(unknown)'}")
            output_lines.append(f"Encrypted: {meta['encrypted']}")
            output_lines.append(f"Size: {meta['file_size_bytes']:,} bytes")
            output_lines.append("")

    # Text extraction
    if args.text or args.all:
        text = extract_text(doc)
        result["text"] = text
        if not args.json:
            output_lines.append("=== Extracted Text ===")
            output_lines.append(text)
            output_lines.append("")

    # OCR text extraction
    if args.ocr:
        ocr_text = extract_text_ocr(pdf_path)
        result["ocr_text"] = ocr_text
        if not args.json:
            output_lines.append("=== OCR Text ===")
            output_lines.append(ocr_text)
            output_lines.append("")

    # List images
    if args.images or args.all:
        images = list_images(doc)
        result["images"] = images
        if not args.json:
            output_lines.append(f"=== Images ({len(images)} found) ===")
            for img in images:
                output_lines.append(f"  Page {img['page']}, Image {img['index']}: {img['width']}x{img['height']} {img['format'].upper()} ({img['size_bytes']:,} bytes)")
            output_lines.append("")

    # Extract images
    if args.extract_images:
        extracted = extract_images(doc, Path(args.extract_images), pdf_path)
        result["extracted_images"] = extracted
        if not args.json:
            output_lines.append(f"=== Extracted {len(extracted)} images to {args.extract_images} ===")
            for path in extracted:
                output_lines.append(f"  {path}")
            output_lines.append("")

    doc.close()

    if args.json:
        print(json.dumps(result, indent=2))
    else:
        print("\n".join(output_lines).strip())


if __name__ == "__main__":
    main()
