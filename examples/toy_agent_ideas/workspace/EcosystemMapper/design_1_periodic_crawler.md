# Design 1: The Periodic Crawler (Conservative)

## Purpose
To map a specific software ecosystem (e.g., "MCP Tools", "Rust Web Frameworks") by periodically scanning a fixed list of sources and updating a structured report.

## Loop Structure
1. **Trigger**: Runs every 24 hours via system scheduler (cron).
2. **Read**: Loads a `config.json` containing seed URLs (GitHub trending, specific blogs, aggregators).
3. **Fetch & Parse**: For each URL:
   - Uses `fetch_fetch` to get content.
   - Uses `grep_search` (or regex) to identify links and capitalized terms (potential entities).
4. **Extract**: Simple heuristic extraction of "Entity A -> related to -> Entity B".
5. **Report**: Generates a daily Markdown summary in `workspace/reports/YYYY-MM-DD.md`.

## Tool Usage
- **filesystem**: Read config, write daily reports.
- **fetch**: Retrieve page content.
- **shell**: Execution triggered by cron; use `grep` for simple extraction.
- **memory**: Not used (stateless between runs, relies on file diffs).

## Memory Architecture
- **Stateless**: The agent does not "remember" the graph structure in a database.
- **Persistence**: Relies on the file system. History is just a folder of daily reports.
- **State**: The "current state" of the ecosystem is implicitly the latest report.

## Failure Modes
- **Broken Links**: If seed URLs change, the agent fails to get data. Recovery: Log error, continue to next URL.
- **Format Changes**: If source HTML structure changes, extraction fails. Recovery: None (requires code update).
- **Redundancy**: Will report the same tools every day unless manually diffed.

## Human Touchpoints
- **Configuration**: User manages the `config.json` of seed URLs.
- **Review**: User reads the daily reports to find new insights.
