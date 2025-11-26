# Design 1: The Pattern Matcher (Conservative)

## Purpose
To detect and resolve "Configuration Drift" and "Copy-Paste Duplication" across multiple local repositories. It aims to reduce maintenance burden by identifying identical files (like `.gitignore`, `.eslintrc`, utility scripts) that have slightly diverged over time.

## Loop Structure
1.  **Discovery Phase**: The agent iterates through a user-defined list of root directories.
2.  **Fingerprinting**: It reads key configuration files and utility helpers, computing content hashes.
3.  **Cluster Analysis**: It uses the **Memory Graph** to group files by similarity (e.g., "tsconfig.json variants").
4.  **Drift Detection**: It identifies the "Dominant Strain" (most common version) and "Mutants" (outliers).
5.  **Reporting**: It generates a Markdown report in a central `inbox/` folder.

## Tool Usage
*   `filesystem`: To traverse projects and read content.
*   `memory`: To store file metadata (Path, Hash, Type, Project) and link variants.
*   `shell`: To run `diff` or `md5sum`.

## Memory Architecture
*   **Nodes**: `Project`, `FileArtifact`, `ContentHash`.
*   **Edges**: `CONTAINS`, `MATCHES_HASH`, `DIVERGES_FROM`.

## Failure Modes
*   **False Positives**: Flagging intentional differences.
*   **Recovery**: The agent maintains an "Ignore List" in memory.

## Human Touchpoints
*   **Read-Only**: The agent never modifies code. It only writes reports.
