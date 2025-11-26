# Design 2: The Context Resurrector

## Purpose
To fight "Context Rot" by autonomously recovering the original source, context, and meaning of orphaned images in a documentation base (e.g., "What does this unlabeled architecture diagram actually represent?").

## Loop Structure
1. **Selection**: Identify images with missing ALT text or low surrounding text density.
2. **Investigation**: 
   - Reverse Image Search to find the image on the web.
   - If found (e.g., in a blog post, whitepaper, or StackOverflow answer), fetch that page.
   - Extract the *surrounding text* (caption, paragraph) from the external source.
3. **Enrichment**: 
   - Create a "Sidecar" metadata file (e.g., `image.png.meta`) containing the origin URL, original caption, and a summary of the concept.
   - Optionally, propose a Pull Request to update the Markdown ALT text with this new context.
4. **Graphing**: Link the image to the *Concepts* it represents in the Memory Graph (e.g., "Image_01" -> "Database Sharding").

## Tool Usage
- **web_brave_image_search**: To find the image online.
- **web_brave_web_search**: To verify the credibility of the source domain.
- **fetch**: To download the external page and extract text.
- **memory**: To build a "Visual Knowledge Graph" (Image -> Concept).

## Memory Architecture
- **Entities**: `Image`, `Concept`, `ExternalDocument`.
- **Relations**: `ExternalDocument` -> `EXPLAINS` -> `Image`.
- **Persistence**: Heavy reliance on Memory Graph to connect visual assets to semantic concepts across the project.

## Failure Modes
- **Hallucination**: Attributing an image to a random Pinterest board instead of the original author.
- **Mitigation**: Prioritize "Canonical" domains (official docs, academic papers) over aggregators.
- **Broken Links**: If the image is unique and not on the web, the agent marks it as "truly proprietary" or "orphan."

## Human Touchpoints
- **Confirmation**: The agent proposes context ("Is this a diagram of the 2023 Architecture?"), user confirms.
- **Seeding**: User points the agent to a specific URL to "ingest" all images and their context.
