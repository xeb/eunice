# Design 1: The "Diff Watcher" (Conservative)

## Purpose
A reliable, low-hallucination agent that monitors competitor public documentation and pricing pages for changes, alerting the team to potential strategic shifts without attempting complex semantic interpretation.

## Loop Structure
1.  **Configuration**: User defines a list of competitor URLs and a crawl frequency in a JSON config file.
2.  **Fetch & Snapshot**: Agent downloads the current HTML/Text of these pages.
3.  **Normalization**: Strips dynamic content (ads, timestamps, session IDs) to create a stable "Canonical Content" signature.
4.  **Comparison**: Diffs the new snapshot against the previous stored snapshot.
5.  **Reporting**: If the diff exceeds a threshold or contains specific keywords (e.g., "New", "Pricing", "Enterprise"), it generates a `diff_report_YYYY-MM-DD.md` in the `workspace/competitor_intel/` folder.

## Tool Usage
*   **web**: Used only for initial discovery of sub-pages (optional).
*   **fetch**: Primary tool for downloading page content.
*   **filesystem**: Stores the "Last Known Good" snapshots and the generated reports.
*   **grep**: Used to scan the diffs for high-priority keywords defined by the user.

## Memory Architecture
*   **Filesystem-based**: Relies primarily on a `snapshots/` directory structure.
*   **Memory Graph**: Minimal usage, perhaps just tracking the "Last Checked" timestamp and "Health Status" of the URLs (to detect 404s).

## Failure Modes
*   **False Positives**: Dynamic CSS or AB testing triggers alerts. (Mitigation: Aggressive DOM cleaning/normalization).
*   **Blocking**: Competitors block the scraper. (Mitigation: User-Agent rotation or slower crawl rates).

## Human Touchpoints
*   **Setup**: User must provide the specific URLs to watch.
*   **Review**: Humans must read the diff reports to decide if the change matters.
