#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "pillow>=10.0.0",
#     "pytesseract>=0.3.10",
#     "google-genai>=1.0.0",
# ]
# ///
"""
Image analysis tool - extracts metadata, performs OCR, and AI description.

Usage:
    uv run analyze.py <image_path> [--ocr] [--metadata] [--describe] [--all]

Examples:
    uv run analyze.py screenshot.png --all
    uv run analyze.py photo.jpg --metadata
    uv run analyze.py document.png --ocr
    uv run analyze.py image.jpg --describe "What objects are in this image?"

Requires GEMINI_API_KEY environment variable for --describe.
"""

import sys
import json
import argparse
from pathlib import Path

def get_metadata(image_path: str) -> dict:
    """Extract image metadata using Pillow."""
    from PIL import Image
    from PIL.ExifTags import TAGS

    img = Image.open(image_path)

    metadata = {
        "filename": Path(image_path).name,
        "format": img.format,
        "mode": img.mode,
        "size": {"width": img.width, "height": img.height},
        "info": {}
    }

    # Get EXIF data if available
    exif_data = img.getexif()
    if exif_data:
        exif = {}
        for tag_id, value in exif_data.items():
            tag = TAGS.get(tag_id, tag_id)
            if isinstance(value, bytes):
                value = value.decode('utf-8', errors='ignore')
            exif[tag] = str(value)
        metadata["exif"] = exif

    # Get other info
    for key, value in img.info.items():
        if isinstance(value, (str, int, float, bool)):
            metadata["info"][key] = value

    return metadata

def perform_ocr(image_path: str) -> str:
    """Extract text from image using Tesseract OCR."""
    try:
        import pytesseract
        from PIL import Image

        img = Image.open(image_path)
        text = pytesseract.image_to_string(img)
        return text.strip()
    except Exception as e:
        return f"OCR failed: {e}\n\nNote: Tesseract must be installed on your system.\nInstall with: apt install tesseract-ocr (Linux) or brew install tesseract (macOS)"

def get_mime_type(image_path: str) -> str:
    """Get MIME type from file extension."""
    ext = Path(image_path).suffix.lower()
    mime_types = {
        ".jpg": "image/jpeg",
        ".jpeg": "image/jpeg",
        ".png": "image/png",
        ".gif": "image/gif",
        ".webp": "image/webp",
        ".bmp": "image/bmp",
    }
    return mime_types.get(ext, "image/jpeg")

def describe_image(image_path: str, prompt: str = "Describe this image in detail.") -> str:
    """Use Gemini 3 Flash Preview to describe the image."""
    import os

    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        return "Error: GEMINI_API_KEY environment variable not set."

    try:
        from google import genai
        from google.genai import types

        with open(image_path, 'rb') as f:
            image_bytes = f.read()

        client = genai.Client(api_key=api_key)
        response = client.models.generate_content(
            model='gemini-2.0-flash',
            contents=[
                types.Part.from_bytes(
                    data=image_bytes,
                    mime_type=get_mime_type(image_path),
                ),
                prompt
            ]
        )

        return response.text
    except Exception as e:
        return f"AI description failed: {e}"

def main():
    parser = argparse.ArgumentParser(description="Analyze images - metadata, OCR, and AI description")
    parser.add_argument("image", help="Path to the image file")
    parser.add_argument("--ocr", action="store_true", help="Perform OCR text extraction")
    parser.add_argument("--metadata", action="store_true", help="Extract image metadata")
    parser.add_argument("--describe", nargs="?", const="Describe this image in detail.",
                        help="AI description using Gemini (optional custom prompt)")
    parser.add_argument("--all", action="store_true", help="Perform all analyses")
    parser.add_argument("--json", action="store_true", help="Output as JSON")

    args = parser.parse_args()

    if not Path(args.image).exists():
        print(f"Error: File not found: {args.image}", file=sys.stderr)
        sys.exit(1)

    # Default to metadata + ocr if no specific option given (not describe, requires API key)
    if not (args.ocr or args.metadata or args.describe):
        args.metadata = True
        args.ocr = True

    # --all enables everything including describe
    if args.all:
        args.metadata = True
        args.ocr = True
        if args.describe is None:
            args.describe = "Describe this image in detail."

    results = {}

    if args.metadata:
        try:
            results["metadata"] = get_metadata(args.image)
        except Exception as e:
            results["metadata_error"] = str(e)

    if args.ocr:
        results["ocr_text"] = perform_ocr(args.image)

    if args.describe:
        results["ai_description"] = describe_image(args.image, args.describe)

    if args.json:
        print(json.dumps(results, indent=2))
    else:
        if "metadata" in results:
            meta = results["metadata"]
            print(f"=== Image Metadata ===")
            print(f"File: {meta['filename']}")
            print(f"Format: {meta['format']}")
            print(f"Size: {meta['size']['width']}x{meta['size']['height']}")
            print(f"Mode: {meta['mode']}")
            if "exif" in meta and meta["exif"]:
                print(f"\nEXIF Data:")
                for k, v in list(meta["exif"].items())[:10]:
                    print(f"  {k}: {v}")
            print()

        if "ocr_text" in results:
            print(f"=== OCR Text ===")
            text = results["ocr_text"]
            if text:
                print(text)
            else:
                print("(No text detected)")
            print()

        if "ai_description" in results:
            print(f"=== AI Description ===")
            print(results["ai_description"])

if __name__ == "__main__":
    main()
