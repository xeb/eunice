# Design 1: The License Auditor

## Purpose
To mitigate legal and reputational risk by autonomously verifying the licensing status of every visual asset (images, diagrams, icons) in a codebase or documentation site. It acts as a "Copyright Linter."

## Loop Structure
1. **Discovery**: Recursively scan the `src/`, `docs/`, and `public/` directories for image files (.png, .jpg, .svg).
2. **Fingerprinting**: Generate a perceptual hash (pHash) or standard hash (SHA-256) for each image.
3. **Verification**: 
   - Perform a Reverse Image Search via Brave API.
   - Analyze results to identify "Stock Photo" sites (Shutterstock, Getty) or "Free" sites (Unsplash, Pexels).
   - Check if the project has a license key or attribution for these assets in a `LICENSES.md` file.
4. **Reporting**: Generate a `audit_report.md` flagging:
   - "High Risk": Images found on paid stock sites with no local license record.
   - "Unknown": Images with no matches (likely custom screenshots).
   - "Safe": Creative Commons images with proper attribution found in the codebase.

## Tool Usage
- **shell**: `find . -name "*.png"` to locate files.
- **filesystem**: Read file bytes for hashing.
- **web_brave_image_search**: To find the image's origin.
- **grep**: To search local Markdown/HTML files for attribution text.

## Memory Architecture
- **Entities**: `Asset` (the file), `Source` (the URL found), `License` (the legal status).
- **Relations**: `Asset` -> `HAS_SOURCE` -> `Source`.
- **Persistence**: A local `assets.json` ledger is sufficient, but a Memory Graph allows tracking "Same image, different file path" duplicates.

## Failure Modes
- **False Positives**: Flagging a generic icon as a paid asset because it appears on a stock site (even if it's open source).
- **Rate Limits**: Brave Search API limits.
- **Recovery**: Pause and resume; allow users to manually "Whitelist" hashes in a config file.

## Human Touchpoints
- **Review**: The agent only *reports*; it never deletes files.
- **Whitelisting**: Humans must explicitly claim "I created this" for unknown assets.
