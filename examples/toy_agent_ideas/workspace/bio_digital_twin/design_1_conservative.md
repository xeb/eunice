# Design 1: The Health Data Warehouse (Conservative)

## Purpose
A passive, privacy-first aggregator that unifies fragmented personal health data (wearables, dietary logs, lab results) into a coherent, locally-stored Knowledge Graph. It solves the "Data Silo" problem where insights are trapped in proprietary apps.

## Loop Structure
1.  **Ingest**: Periodically scans `inbox/health_data/` for new exports (CSV, JSON, XML) from supported providers (Apple Health, Oura, Cronometer).
2.  **Normalize**: Standardizes units and timestamps using local Python scripts.
3.  **Graphing**: Creates nodes for `Metric` (e.g., "Resting Heart Rate"), `Observation` (Value + Time), and `Event` (e.g., "Run 5k").
4.  **Reporting**: Generates a weekly Markdown summary in `outbox/weekly_health.md` highlighting trends (e.g., "Sleep efficiency down 5% this week").

## Tool Usage
*   **filesystem**:
    *   Reading raw data exports from `inbox/`.
    *   Writing summary reports to `outbox/`.
    *   Archiving processed files to `archive/`.
*   **memory**:
    *   Storing the ontology of metrics (e.g., `(Caffeine) --increases--> (Heart Rate)`).
    *   Storing the time-series data as a graph of linked observations for semantic querying.
*   **grep**:
    *   Searching through large CSVs for specific timestamps or tags before ingestion.

## Memory Architecture
*   **Entities**: `Metric`, `Observation`, `Activity`, `Substance`.
*   **Relations**: `HAS_VALUE`, `OCCURRED_AT`, `CORRELATED_WITH` (statistical).
*   **Persistence**: Strict separation—Personal identifiable data (PID) stays in local `filesystem` or local `memory`. No external transmission.

## Failure Modes
*   **Format Drift**: API exports change schema. Agent detects parsing errors and flags the file to `errors/` for human review.
*   **Volume**: Graph becomes too large. Strategy: Prune granular data (minute-by-minute HR) into daily aggregates after 90 days.

## Human Touchpoints
*   **Setup**: User defines data sources and import paths.
*   **Review**: User reads weekly Markdown reports.
*   **Maintenance**: User fixes schema errors when providers change formats.
