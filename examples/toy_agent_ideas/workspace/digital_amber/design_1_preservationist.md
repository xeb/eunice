# Design 1: The Preservationist (Conservative)

## Purpose
A "do no harm" background agent that creates local mirrors of every external link found in your notes, ensuring that if the internet disappears tomorrow, your knowledge base survives. It modifies your files minimally by appending a local link.

## Loop Structure
1. **Watch**: Monitors a specific directory (e.g., `~/Notes`) for file changes using `filesystem`.
2. **Extract**: On change, uses `grep` to find all HTTP/HTTPS links in the modified file.
3. **Check**: Queries `memory` to see if this URL has already been archived.
4. **Archive**: If new, uses `fetch` (or `shell` invoking `wget`/`single-file`) to download a self-contained HTML/PDF snapshot to `~/Notes/_archive/<hash>.html`.
5. **Record**: Updates `memory` with URL -> Archive Path mapping.
6. **Annotate**: Appends a discreet `[local]` link next to the original link in the Markdown file using `text-editor`.

## Tool Usage
- **filesystem**: Reading notes, writing archive files.
- **fetch**: Downloading content (lightweight).
- **shell**: Calling robust tools like `monolith` or `pandoc` for high-fidelity saving.
- **memory**: Storing the "Manifest" of archived URLs (URL, Date, LocalPath, Hash).
- **text-editor**: Inserting the archive link non-destructively.

## Memory Architecture
- **Entities**: `URL`, `ArchiveSnapshot`
- **Relations**: `URL hasSnapshot ArchiveSnapshot`
- **Observations**: "Last checked status", "Content Hash".

## Failure Modes
- **Download Fails**: Site has anti-bot protection. -> Mark as "Unarchivable" in memory, do not retry often.
- **Disk Space**: Archives grow too large. -> Implement quota or old-archive pruning.
- **File Corruption**: Archive file is empty. -> Verify size > 0 before linking.

## Human Touchpoints
- **Configuration**: User sets "Archive Folder" and "Max Cache Size".
- **Visual**: User sees `[local]` links appear in their notes automatically.
