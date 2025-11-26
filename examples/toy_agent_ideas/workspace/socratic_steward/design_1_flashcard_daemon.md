# Design 1: The Flashcard Daemon

## Purpose
A conservative, maintenance-focused agent that automatically converts your static notes into active study materials. It ensures you retain what you write by turning Markdown headers and definitions into Spaced Repetition (SRS) cards.

## Loop Structure
1. **Watch**: Monitors a specific 'notes/' directory for file changes.
2. **Parse**: When a file is modified, it scans for specific patterns (e.g., `Term:: Definition` or `Q: ... A: ...`).
3. **Generate**: Extracts these pairs and compiles them into a daily review file (e.g., `review_2025-11-25.md`) or exports to Anki CSV.
4. **Clean**: Updates the source file metadata to indicate when it was last "harvested."

## Tool Usage
* **filesystem**: Reading notes, writing review files, tracking last-modified timestamps.
* **grep**: Efficiently finding extraction patterns across thousands of files.
* **memory** (Optional): Storing the "next review date" for each item if implementing a custom SRS algorithm.

## Memory Architecture
* **Filesystem-based persistence**: The state is stored in the notes themselves (via YAML frontmatter) or in a sidecar JSON file. This is robust and portable.

## Failure Modes
* **Parse Errors**: Poorly formatted notes might generate garbage cards. *Recovery*: Skip malformed lines and log a warning in a `parsing_errors.log` file.
* **Overload**: Too many cards generated at once. *Recovery*: Cap daily new cards to a fixed number (e.g., 20).

## Human Touchpoints
* **Review**: The human actively interacts with the generated daily review file.
* **Correction**: If a card is wrong, the human edits the *source* note, not the card, enforcing a "single source of truth."
