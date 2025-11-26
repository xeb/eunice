# Design 3: The Asset Curator

## Purpose
To actively maintain and upgrade the *quality* of visual assets. It treats images as "binary dependencies" that can be outdated, deprecated, or low-resolution.

## Loop Structure
1. **Assessment**: Analyze local images for quality metrics (resolution, format, compression artifacts).
2. **Search**: 
   - Reverse search for *higher resolution* versions of the same image (e.g., finding the original SVG of a blurry PNG logo).
   - Search for "modern versions" of detected UI elements (e.g., detecting an iOS 6 screenshot in 2024 docs).
3. **Upgrade Proposal**:
   - Download the better asset to a temporary staging folder.
   - Generate a "Visual Diff" (side-by-side comparison).
   - Create a Pull Request: "chore: Upgrade 'logo.png' to high-res SVG found at [Official Brand Site]."
4. **Deduplication**: Identify identical images stored under different names and refactor the markdown to point to a single source.

## Tool Usage
- **filesystem**: Read file headers, check sizes.
- **web_brave_image_search**: Search with "large" size filter.
- **shell**: Use `imagemagick` (if available) or standard file operations to compare.
- **memory**: Track "Attempted Upgrades" to avoid pestering the user about the same low-res image.

## Memory Architecture
- **Entities**: `AssetVersion`.
- **Relations**: `Asset A` -> `IS_BETTER_VERSION_OF` -> `Asset B`.
- **Persistence**: File-system heavy (staging areas), Memory light (just history).

## Failure Modes
- **Semantic Drift**: Replacing a specific "v1.0 screenshot" with a "v2.0 screenshot" when the docs *meant* to show v1.0.
- **Mitigation**: Strict "Visual Similarity" threshold and explicit human approval for any content change.

## Human Touchpoints
- **Gatekeeper**: Humans must approve the "Upgrade" PR.
- **Directives**: Humans can tag folders as "Legacy" to prevent auto-upgrading.
