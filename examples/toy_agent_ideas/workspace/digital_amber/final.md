# Agent: The Digital Amber

**"The internet is written in ink that disappears."**

## Core Concept
**The Digital Amber** is a background preservation agent that ensures your personal knowledge base is immune to link rot. It treats every external URL in your local notes as a liability until it is secured. It proactively creates local, content-addressable snapshots of referenced web pages and transparently links them in your documents. If a live link dies, it acts as a "Resurrector," finding mirrors and patching your notes.

## Architecture

### 1. The Watcher (Filesystem Loop)
- **Trigger**: File modification in `~/Notes` or `~/Projects`.
- **Action**: Scans for `http(s)://` patterns.
- **Logic**:
    - Has this URL been seen? (Check `memory` graph).
    - If no, add to `DownloadQueue`.
    - If yes, verify `SnapshotPath` exists.

### 2. The Fossilizer (Preservation Loop)
- **Trigger**: Item in `DownloadQueue`.
- **Action**:
    - Uses `fetch` (or `single-file` CLI tool) to download a standalone HTML/PDF.
    - Saves to `~/Notes/_archive/<Year>/<Domain>/<Title>_<Hash>.html`.
    - Updates `memory`:
        - Entity: `UrlNode`
        - Property: `local_path`, `last_crawled`, `content_hash`.
    - **Optimization**: Uses `grep` to check if the file size is valid (not an empty error page).

### 3. The Resurrector (Maintenance Loop)
- **Trigger**: Periodic schedule (e.g., Weekly) or User Request.
- **Action**: Checks health of live URLs.
- **Logic**:
    - If `HTTP 404` or `DNS NXDOMAIN`:
        - Check `memory` for a local snapshot.
        - If local snapshot exists: Suggest replacing link with relative local path.
        - If no local snapshot: Search `web` (Internet Archive / Google Cache).
        - Generate a `Maintenance_Report.md` in the root folder with checkboxes:
            - `[ ] Fix: http://dead.link -> Use Snapshot (2024-05-01)`
            - `[ ] Fix: http://moved.link -> Use Wayback Machine`

## Tool Usage
- **filesystem**:
    - `read_text_file`: parsing notes.
    - `write_file`: saving archives.
    - `search_files`: finding broken links across valid extensions (.md, .txt).
- **fetch**:
    - Downloading content.
    - Performing `HEAD` requests for health checks.
- **web**:
    - `brave_web_search`: Finding new locations for moved content.
    - `brave_web_search` (site:archive.org): Finding snapshots.
- **memory**:
    - Storing the "Link Ledger" (URL -> Local Path mapping).
    - Avoiding re-downloading the same URL across multiple files.
    - Tracking "Dead Link" status to avoid repeated 404 pings.

## Persistence Strategy
**Hybrid**:
- **Filesystem**: The "Truth" is the `_archive/` folder. It is portable, standard (HTML/PDF), and survives without the agent.
- **Memory Graph**: optimization layer. Tracks which URLs are in which files to allow for fast "impact analysis" (e.g., "This one link is used in 50 files, we must fix it").

## Autonomy Level
**High Autonomy (Preservation) / Checkpoint (Repair)**
- **Preservation**: Fully autonomous. It is always safe to save a copy.
- **Repair**: Human-in-the-loop. The agent proposes fixes in a Markdown report; the user confirms (or the agent can be configured to "Auto-fix safe redirects").

## Failure Modes & Recovery
1. **IP Blocking**: Agent detects "403 Forbidden" or CAPTCHA.
    - *Response*: Logs to `Excluded_Domains` list in memory. Stops trying that domain for 24h.
2. **Disk Bloat**: Archive folder grows too big.
    - *Response*: Agent checks `filesystem_list_directory_with_sizes`. If > Quota, it suggests deleting oldest/largest snapshots or converting to text-only.
3. **False 404s**: Temporary downtime.
    - *Response*: "Resurrector" requires 2 failures over 24h before declaring a link dead.

## Key Insight
Most "Bookmarking" tools are **destinations** (you go to them to save). The Digital Amber is **infrastructure**—it lives *under* your existing workflow. You just write Markdown as usual; the agent ensures the links you write become permanent records, effectively turning your personal notes into a self-contained internet.
