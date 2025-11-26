# Design 3: The Bit-Rot Excavator

## Purpose
To actively "restore" digital artifacts by migrating them from obsolete formats (Fossils) to modern standards, preventing data loss due to software extinction.

## Problem Domain
"Digital Dark Age": File formats die. Code dependencies vanish from NPM/PyPI. If you don't migrate data/code forward, it becomes unreadable. This agent acts as an autonomous museum conservator.

## Core Toolset
*   **shell**: Running conversion tools (ffmpeg, pandoc, codemods).
*   **filesystem**: Managing "Excavation Sites" (staging folders).
*   **web**: Searching for "Rosetta Stones" (old documentation/converters).

## Loop Structure
1.  **Survey**: Scans storage for "Endangered Species" (file extensions like .flash, .py2, or dependencies on deprecated APIs).
2.  **Risk Assessment**: Checks if the tools to read these formats are still available in the current OS environment.
3.  **Excavation**:
    *   Copies the artifact to a `staging/excavation` folder.
    *   Spins up a container or virtual environment (if needed/possible).
    *   Attempts a "Transmutation" (Migration) using standard tools (e.g., `2to3`, `ffmpeg`).
4.  **Verification**: Checks if the output is readable and retains key properties of the input.
5.  **Preservation**: Stores the new version alongside the original (which is kept as an immutable artifact).

## Memory Architecture
*   **Nodes**: `Artifact`, `Format`, `MigrationPath`.
*   **Edges**: `CAN_MIGRATE_TO` (Format A -> Format B).
*   **Properties**: `conservation_status` (Safe, Endangered, Extinct).

## Failure Modes
*   **Destructive Migration**: Conversion loses data (lossy compression). *Recovery:* Always keep original; requires human sign-off to delete.
*   **Tool Rot**: The migration tool itself is broken. *Recovery:* Web search for alternatives.

## Human Touchpoints
*   **Appraisal**: Human decides if an artifact is worth saving.
*   **Quality Check**: Human verifies the converted file fidelity.
