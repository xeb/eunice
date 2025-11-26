# Design 3: The Art Director (Hybrid/Collaborative)

## Purpose
A tool-using agent that doesn't just "find" images but "composes" them. It acts as an intelligent filter and layout engine, using local shell tools (ImageMagick) to normalize, palette-swap, or grid images into cohesive "Style Guides".

## Loop Structure
1. **Briefing**: User creates a folder `project_alpha/` and drops a `style_brief.md` (e.g., "Palette: #FF00FF, #000000; Vibe: Glitch Art").
2. **Sourcing**:
   - Agent searches web for images matching the vibe.
   - **Crucial Step**: It also scans the *local* filesystem for existing assets that match.
3. **Processing (Shell)**:
   - Uses `shell_execute_command` with `magick` (ImageMagick) to extract the dominant colors of downloaded images.
   - Filters out images that clash with the brief's palette.
   - Generates "Contact Sheets" (grid compilations) of the survivors.
4. **Refinement**:
   - Updates a `project_alpha/assets/index.md` with the contact sheets.
   - asks for feedback: "I found 50 images, filtered to 12 that match your palette. Keep searching?"

## Tool Usage
- **shell**: Heavily used for `magick` identify/convert operations to analyze image histograms.
- **filesystem**: organizing assets into "Candidate" vs "Approved" folders.
- **web**: Source material.
- **memory**: Stores the "Project Palette" and "Rejected hashes" (to avoid redownloading same bad images).

## Memory Architecture
- **Project-Scoped**:
  - `Project("Alpha")` has `Palette([...])`.
  - `Image("hash123")` has `Status("Rejected")`.

## Failure Modes
- **Tool Missing**: ImageMagick not installed.
  - *Recovery*: Fallback to simple metadata filtering (keywords) or prompt user to install.
- **Empty Set**: No images match the strict palette.
  - *Recovery*: Relax the color distance threshold automatically.

## Human Touchpoints
- **Review**: The agent produces a "Contact Sheet" (image grid). The user deletes the ones they hate.
- **Steer**: The user modifies the `style_brief.md` to change the target palette.
