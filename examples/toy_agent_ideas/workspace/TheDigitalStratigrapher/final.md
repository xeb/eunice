# Agent: The Digital Stratigrapher

## Abstract
**The Digital Stratigrapher** is an autonomous "Geologist for Software" that maps the evolutionary history of long-running systems. Instead of treating a repository as a flat collection of files, it treats it as a sedimentary formation with distinct "Geological Eras" (e.g., "The jQuery Epoch", "The Pre-GDPR Era"). It prevents "Anachronistic Failures" by warning developers when they attempt to use modern tools on "Fossilized" code or data without proper excavation techniques.

## Problem Domain
*   **Schema Drift:** Old database records missing fields required by new code.
*   **Bit Rot:** Code that technically compiles but relies on deprecated mental models (e.g., callbacks vs promises).
*   **Institutional Amnesia:** "Why is this column here?" (Answer: It was required by a partnership that ended in 2018).

## Core Toolset
*   **filesystem**: Scanning codebase and data samples.
*   **memory**: Storing the **"Stratigraphic Matrix"** (Graph of Eras, Fossils, and Relations).
*   **grep**: Identifying "Index Fossils" (unique patterns defining an era).
*   **shell**: Running `git log`, `jq`, or AST parsers to date-stamp strata.

## Architecture

### 1. The Survey Loop (Autonomous)
The agent runs in the background, performing a "Geological Survey" of the repo:
*   **Index Fossil Detection**: It identifies unique signatures of specific time periods.
    *   *Example:* `var self = this` -> **The ES5 Era (2009-2015)**.
    *   *Example:* `"created_at": "Wed, 02 Oct 2002..."` -> **The RFC 822 Era**.
*   **Stratification**: It groups files and data records into **"Stratigraphic Units"** (Contexts).
*   **Map Generation**: It builds a persistent graph in Memory where every file is linked to its **Provenience** (Era).

### 2. The Excavation Loop (On-Demand)
When a user opens a file or queries data:
*   **Contextual Warning**: "You are editing a file from the *AngularJS 1.x Stratum*. Note that dependency injection works differently here."
*   **Anachronism Check**: If the user adds a modern import (e.g., React Hooks) to a Fossil file, the agent warns of an **"Unconformity"** (mixing incompatible eras).

### 3. The Dating Loop (Historical)
*   **Cross-Reference**: It uses `git log` to calibrate its "Fossils" against absolute time.
*   **Drift Detection**: It monitors live data for "New Sediment" (schema changes) and updates the current Era definition.

## Memory Graph Structure (The Matrix)
*   **Nodes**:
    *   `Era` (e.g., "Legacy Auth System").
    *   `IndexFossil` (e.g., Pattern `require('request')`).
    *   `Stratum` (A set of files belonging to an Era).
*   **Edges**:
    *   `SUCCEEDS` (Era A came after Era B).
    *   `CONTAINS` (Stratum contains File).
    *   `IS_INCOMPATIBLE_WITH` (Era A crashes if combined with Tool B).

## Use Cases
*   **Safe Modernization**: "Show me all files in the 'Callback Era' so I can prioritize refactoring."
*   **Data Debugging**: "Why did the import fail?" -> "Record #405 belongs to the 'V1 Schema' which lacked the 'email' field."
*   **Onboarding**: Helping new devs understand *why* the code looks so inconsistent.

## Failure Modes & Recovery
*   **Misidentification**: A file might look old but be a modern polyfill.
    *   *Recovery:* User explicitly tags the file as "Pseudo-Fossil" in the graph.
*   **Mixed Strata**: A single file containing 10 years of edits.
    *   *Recovery:* The agent segments the file by line ranges, visualizing the "Sedimentation" within the file.

## Human Interaction
*   **"Naming the Era"**: The agent detects a cluster of changes; the human names it (e.g., "The AWS Migration").
*   **"Excavation Permit"**: When deleting massive amounts of "Fossil" code, the agent asks for confirmation to "Destroy the Archaeological Record."
