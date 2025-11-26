# Design 1: The Schema Historian

## Purpose
To autonomously map the evolutionary history of data structures in long-running systems, enabling developers to understand "Data Stratigraphy" (how data shapes have changed over time) and prevent "Time-Travel Bugs" where modern code fails on ancient records.

## Problem Domain
In systems running for years, data formats drift. A JSON blob from 2019 looks different from 2024. Code usually handles the *current* schema well, but crashes or corrupts data when encountering "Fossils" (old records) during batch jobs or migrations.

## Core Toolset
*   **filesystem**: Scanning data lakes, logs, and local DB dumps.
*   **memory**: Storing the "Schema Phylogeny" (family tree of data shapes).
*   **grep**: Searching for timestamp patterns to correlate data with eras.
*   **shell**: Executing `jq` or python scripts for parsing.

## Loop Structure
1.  **Survey**: Agent iterates through data files (JSON, CSV, Log, SQL dump).
2.  **Sampling**: Reads `N` random records from different file timestamps.
3.  **Inference**: Infers the schema (keys, types) for each sample.
4.  **Stratification**:
    *   Compares inferred schema with the "Known Schema Graph".
    *   If distinct, creates a new "Stratigraphic Unit" (Era) in memory.
    *   Identifies the "Index Fossil" (the specific field or value that defines this Era, e.g., `"user_id": int` vs `"user_id": uuid`).
5.  **Mapping**: Tags file paths or ID ranges with their specific Era.
6.  **Reporting**: Generates a "Harris Matrix" visual showing the timeline of schema changes.

## Memory Architecture
*   **Nodes**: `SchemaEra`, `Field`, `DriftEvent`.
*   **Edges**: `EVOLVED_FROM` (Schema A -> Schema B), `CONTAINS` (Era -> File).
*   **Properties**: `index_fossil` (discriminator), `date_range`.

## Failure Modes
*   **Sampling Bias**: Misses rare schema variants if sampling is too small. *Recovery:* User can force full scan or provide "problematic" examples.
*   **Encryption/Binary**: Cannot parse opaque formats. *Recovery:* Flags as "Unexcavated Bedrock".

## Human Touchpoints
*   **Ratification**: User confirms that a detected change is a legitimate "Era" and not just bad data.
*   **Naming**: User provides semantic names for Eras (e.g., "Pre-GDPR", "Legacy Auth").
