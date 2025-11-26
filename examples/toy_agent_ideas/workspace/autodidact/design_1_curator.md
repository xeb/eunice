# Design 1: The Curriculum Curator (Conservative)

## Purpose
A "Smart Librarian" that automates the collection and organization of learning resources. Instead of 50 open tabs, the user gets a structured set of directories and reading lists.

## Loop Structure
1. **Trigger:** User runs `autodidact learn "Rust Programming"`.
2. **Search:** Agent performs web searches for "Rust programming roadmap", "best rust tutorials 2024", "Rust book".
3. **Filter:** Agent fetches the pages and extracts links, title, and difficulty level (heuristic based on text).
4. **Organize:** Agent creates a directory structure:
   - `Rust_Programming/`
     - `01_Beginner/`
     - `02_Intermediate/`
     - `03_Advanced/`
     - `References.md`
5. **Report:** Agent writes a `README.md` in the root with a suggested reading order.

## Tool Usage
- **web:** `web_brave_web_search` for finding resources.
- **filesystem:** `create_directory` and `write_file` (simulated via shell) for organizing content.
- **fetch:** `fetch_fetch` to grab page summaries/metadata.

## Memory Architecture
- **Stateless (Filesystem only):** The state is entirely contained in the directory structure. If the user deletes a folder, the "memory" is lost.
- **Benefits:** Simple, portable, user-readable.

## Failure Modes
- **Link Rot:** Saved links die. (Recovery: Agent can be asked to "refresh" a folder).
- **Bad Taxonomy:** Agent puts advanced topics in beginner folders. (Recovery: User manually moves files).

## Human Touchpoints
- **Curated Selection:** User can delete unwanted resources.
- **Refresh:** User requests updates.
