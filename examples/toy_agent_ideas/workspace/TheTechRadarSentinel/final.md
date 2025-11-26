# Agent: The Tech Radar Sentinel

## One-Line Summary
An autonomous portfolio manager that aligns your codebase's local dependency choices with global technology trends, preventing "rot" by proactively identifying outdated or redundant tools.

## Problem Domain
Codebases naturally entropy. Developers pick libraries that are popular *today*, but 3 years later, those libraries are abandoned or superseded. Manual "Tech Radars" are rarely updated. Teams end up with 5 different HTTP clients or 3 different Date libraries, accumulating "cognitive debt" and security risks.

## Core Insight: "Contextual Relevance Filtering"
Most dependency checkers just look for *version updates* (e.g., "Bump v1.0 to v1.1"). The Tech Radar Sentinel looks for **Conceptual Updates** (e.g., "Replace `request` with `axios`" or "Replace `moment` with `Temporal`"). It achieves this by mapping every dependency to a **Problem Domain** in its Memory Graph and comparing the *Best-in-Class* solution for that domain against what is currently installed.

## Architecture

### 1. The Survey Loop (Filesystem + Grep)
-   **Manifest Parsing:** Recursively finds `package.json`, `pom.xml`, etc.
-   **Usage Density:** Uses `grep` to count import frequency. A library used in 1 file is "Low Risk"; a library used in 500 files is "High Impact".
-   **Output:** A raw inventory of "What we have".

### 2. The Knowledge Loop (Memory + Web)
-   **Ontology Mapping:** Queries the Memory Graph: "What is `zod`?"
    -   *Hit:* Returns `(zod) --[IS_A]--> (Schema Validation)`.
    -   *Miss:* Executes `web_brave_web_search` ("zod library alternative", "zod vs yup") to categorize it, then updates Memory.
-   **Trend Evaluation:** Queries the Memory Graph for the "Best Practice" in that category.
    -   *Hit:* "Standard is `zod`."
    -   *Miss:* Searches "Best Schema Validation 2025". Updates Memory: `(Schema Validation) --[RECOMMENDS]--> (zod)`.

### 3. The Judgment Loop (Logic)
-   **Comparison:**
    -   *Scenario A (Aligned):* Code uses `zod`. Trend recommends `zod`. -> **Status: ADOPT**.
    -   *Scenario B (Drift):* Code uses `joi`. Trend recommends `zod`. -> **Status: ASSESS** (if usage is low) or **HOLD** (if usage is high/legacy).
    -   *Scenario C (Danger):* Code uses `express-validator` (v2). Trend warns "Unmaintained". -> **Status: STOP**.

### 4. The Reporting Loop (Filesystem)
-   **Radar Generation:** Writes a `tech-radar.json` file for visualization.
-   **Strategic Brief:** Generates `ADVICE.md`:
    > **Strategic Recommendation:**
    > We detected **High Usage** of `moment.js` (Deprecated).
    > **Industry Standard:** `date-fns` or `Temporal`.
    > **Action:** Migration Plan generated in `/plans/migrate_moment_to_datefns.md`.

## Persistence Strategy
-   **Memory Graph:** Stores the "World Knowledge" (Library Categories, Industry Trends) so it doesn't need to re-search every time.
-   **Filesystem:** Stores the "Local State" (Radar JSON, Reports).

## Tools Used
1.  **memory:** `memory_create_entities` (Libraries, Categories), `memory_search_nodes`.
2.  **web:** `web_brave_web_search` (Trends, Alternatives).
3.  **filesystem:** `filesystem_read_file` (Manifests), `filesystem_write_file` (Reports).
4.  **grep:** `grep_count_matches` (Usage density).

## Autonomy Level
**High (Observer/Advisor)**. It runs autonomously in the background (e.g., weekly) and submits a PR with the updated Radar and Advice. It does *not* change code automatically, but it *can* generate migration scripts for humans to review.
