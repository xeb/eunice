# Design 2: The Code Chronologist

## Purpose
To map the "Geological Layers" of a codebase based on coding patterns, dependencies, and idioms, identifying "Fossilized" code that poses modernization risks.

## Problem Domain
Codebases are not uniform. They contain "strata" of different styles (e.g., jQuery era, ES5 class era, React Hooks era). Treating them as uniform leads to failed refactors. Developers need to know "How old is the *thinking* in this file?", not just "When was the last commit?".

## Core Toolset
*   **grep / filesystem**: Searching for syntactic markers (e.g., `var`, `React.createClass`, `import`).
*   **memory**: Mapping files to "Style Eras".
*   **shell**: Running `git log` analysis to correlate style with time.

## Loop Structure
1.  **Pattern Mining**: Agent scans the codebase for "Index Fossils" of specific technologies (e.g., `rdd` implies Spark 1.x, `pandas.append` implies older Pandas).
2.  **Stratigraphy**:
    *   assigns a "Geological Age" to every file or function based on the density of these markers.
    *   Detects "Unconformities" (e.g., modern syntax mixed with ancient patterns).
3.  **Risk Mapping**: Generates a "Heatmap of Fossilization".
    *   **Active Layer**: High churn, modern syntax.
    *   **Sedimentary Layer**: Low churn, stable, slightly dated.
    *   **Bedrock/Fossil Layer**: Zero churn, ancient syntax, high risk of breakage if touched.
4.  **Advisory**: When a user opens a "Fossil" file, the agent warns: "You are entering the Callback Era (2016). Modern tools may break here."

## Memory Architecture
*   **Nodes**: `TechStratum`, `FileContext`.
*   **Edges**: `BELONGS_TO_STRATUM`.
*   **Properties**: `marker_density`, `last_disturbance` (commit date).

## Failure Modes
*   **Misdating**: Modern code mimicking old styles (or vice versa). *Recovery:* Uses commit history as a secondary validator.
*   **Poly-Era Files**: Files that have been partially refactored. *Recovery:* Segment files into regions.

## Human Touchpoints
*   **Era Definition**: Users help define the "Index Fossils" for their specific project (e.g., "We switched to TypeScript in 2021").
