# Design 3: The Interactive Notebook Converter (Hybrid)

## Purpose
Instead of just fixing static text, this agent transforms static Markdown documentation into executable Jupyter Notebooks (or simple shell scripts) that users can download and run. It proves the documentation works by *building* the artifact that the documentation describes.

## Core Toolset
- **filesystem:** To read docs and write new notebook artifacts.
- **shell:** To install `jupyter` or test generated scripts.
- **memory:** To track the mapping between "Source Doc" and "Generated Artifact".

## Architecture

### 1. Loop Structure
1.  **Ingest:** Reads `README.md` or `tutorial.md`.
2.  **Contextualize:** Identifies dependencies between code blocks (e.g., Block A sets a variable used in Block B). Uses Memory to track variable scope.
3.  **Synthesize:** Creates a new file (e.g., `tutorial_generated.ipynb` or `run_demo.sh`) containing the code blocks + markdown text.
4.  **Validate:** Executes the *entire generated artifact* from top to bottom.
5.  **Publish/Warn:**
    *   If it runs successfully: Commits the generated artifact to a `examples/` directory.
    *   If it fails: Adds a warning badge to the source Markdown: "⚠️ This tutorial failed automated verification on [Date]."

### 2. Memory Architecture
-   **Dependency Graph:** Tracks how variables flow between code snippets in a document.
-   **Execution State:** Remembers the state of the file system required for the tutorial (e.g., "needs `data.csv` to exist").

### 3. Safety & Failure Modes
-   **Isolation:** Runs validation in a fresh Docker container to ensure no hidden dependencies are assumed ("It works on my machine" prevention).
-   **State Reset:** Cleans up any files created during the validation run.

### 4. Human Touchpoints
-   **Usage:** Humans use the *generated* notebooks as the primary way to consume the tutorial.
-   **Feedback:** If a generated notebook fails for a user, they report it, and the agent attempts to reproduce the failure.

## Pros/Cons
-   **Pros:** Provides immediate value to users (copy-pasteable/runnable code); strongly incentivizes keeping docs green.
-   **Cons:** Hard to handle complex tutorials that require external services (databases, APIs) or GUI interactions.
