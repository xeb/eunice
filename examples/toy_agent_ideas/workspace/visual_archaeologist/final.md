# Agent: The Visual Archaeologist

## High-Level Concept
A background daemon that treats the project's visual assets (images, diagrams, screenshots) not as static binary blobs, but as **citations that need verification**. It autonomously reverse-searches every image in the codebase to build a "Provenance Graph," ensuring legal compliance, recovering lost context (ALT text), and maintaining visual quality.

## Problem Domain
1.  **Legal Risk**: Accidentally shipping copyrighted stock photos or uncredited diagrams.
2.  **Context Rot**: "Mystery Screenshots" in old docs where no one knows what version or system is depicted.
3.  **Visual Debt**: Low-resolution, duplicated, or deprecated assets cluttering the repo.

## The Core Loop
1.  **Inventory & Hash**:
    *   Scan `src/`, `docs/`, `static/` for image files.
    *   Calculate `SHA-256` (identity) and `pHash` (perceptual similarity).
    *   Query Memory Graph: "Do we already know the origin of this hash?"

2.  **Forensic Search (If Unknown)**:
    *   **Reverse Image Search** (via Brave) to find online occurrences.
    *   **Text Extraction** (simulated via OCR or finding the image in HTML contexts online) to understand *what* the image depicts.
    *   **License Check**: Cross-reference the found URL with known "Stock" (Getty/Shutterstock) or "Free" (Unsplash/Wikimedia) domains.

3.  **Contextual Graphing**:
    *   Create **Memory Entities**: `Asset` -> `OriginURL` -> `LicenseStatus`.
    *   Link `Asset` -> `Concept` (e.g., "Architecture Diagram" -> "Microservices").

4.  **Actionable Reporting**:
    *   **Red Flags**: "Found paid stock photo [URL] in `src/assets/bg.jpg` with no license key."
    *   **Enrichment**: "Found high-res source for `logo_blur.png` at [Brand URL]. Upgrade available."
    *   **Auto-Captioning**: Generate a `image.meta.json` sidecar with the original caption found online ("Figure 3 from 2022 Whitepaper").

## Tool Usage
*   **web_brave_image_search**: The primary sensor for finding "Where else does this exist?"
*   **memory**: Stores the "Provenance Graph" (Hash -> Origin -> License).
*   **filesystem**: Reads binaries, writes `audit_report.md` and `.meta` sidecars.
*   **shell**: Calculates hashes.

## Persistence Strategy
**Hybrid**:
*   **Memory Graph**: Stores the "Truth" (Provenance, Trust Scores, License Status).
*   **Filesystem**: Stores the "Evidence" (Sidecar files, Audit Reports) so humans can version-control the findings.

## Autonomy Level
**High Autonomy (Research) / Checkpoint (Action)**:
*   The agent **autonomously** researches and builds the graph in the background.
*   It **never** deletes or replaces files without a human merging a PR.
*   It **autonomously** commits "Sidecar" files (metadata) if configured to do so.

## Key Insight
**"Reverse Image Search for Code"** — Treating images as **External Dependencies** (like npm packages) that require a  (the Provenance Graph) to track their version, license, and source.

## Failure Modes & Recovery
*   **False Positives**: Matching a generic "Checkmark Icon" to a stock photo site.
    *   *Recovery*: User adds `.visualignore` or manually sets metadata `"origin": "internal"`.
*   **Rate Limits**: Search API quotas.
    *   *Recovery*: Exponential backoff and prioritization of "New" assets only.

## Future Expansion
*   **Generative Repair**: If an image is "Unknown" and "Low Quality," use an Image Gen model to recreate it in vector format based on the inferred context.
