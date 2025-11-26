# Design 1: The Librarian (Conservative)

## Purpose
A reliable background daemon that organizes files into a standard hierarchy based on explicit metadata. It aims to eliminate "Downloads folder clutter" without risking data loss or confusing organization.

## Loop Structure
1. **Scan:** Periodically (e.g., hourly) scan a specific "Ingest" directory.
2. **Identify:** Use file extensions and basic header analysis (magic numbers) to identify file types.
3. **Extract:** Use deterministic tools (e.g., `exiftool` equivalent via shell or python scripts) to extract creation date, author, and camera model.
4. **Action:**
   - Rename file to `YYYY-MM-DD_[OriginalName].[ext]`.
   - Move file to `Archive/[Year]/[Month]/[Type]/` (e.g., `Archive/2024/11/Documents/`).
   - Log the action to a text file.
5. **Report:** Generate a daily summary of moved files.

## Tool Usage
- **Filesystem:** 
  - `filesystem_list_directory` to find new files.
  - `filesystem_move_file` to organize them.
  - `filesystem_create_directory` to create date-based folders.
- **Shell:** 
  - `shell_execute_command` to run standard linux tools (`file`, `stat`, `grep`) for metadata extraction.
  - No AI interpretation is used for file operations to ensure 100% safety.

## Memory Architecture
- **Stateless/Log-based:**
  - Does not maintain a complex persistent graph.
  - Relies on the file system structure itself as the "database".
  - Uses a simple JSON log file `processed_history.json` to avoid re-processing files if they are moved back to Ingest.

## Failure Modes
- **Naming Collisions:** Handles duplicates by appending counter `_1`, `_2`.
- **Unknown Types:** Moves unidentified files to `Archive/Unsorted/`.
- **Permission Errors:** Logs error and skips file; does not crash.

## Human Touchpoints
- **Configuration:** User defines the `Ingest` path and `Archive` root in a config file.
- **Review:** User only needs to check the `Unsorted` folder occasionally.
