# Design 1: The Persona Simulator

## Purpose
A command-line tool that allows developers to run their code, documentation, or design specs past a set of pre-defined "Synthetic Personas" to get diverse feedback instantly. This replaces the need to constantly nag colleagues for "sanity checks."

## Loop Structure
1. **Trigger**: User runs `critique <file> --persona <name>`.
2. **Context Loading**: Agent reads the target file and any linked context (imports, references).
3. **Persona Loading**: Agent loads the definition of the selected persona (e.g., "Angry Sysadmin", "Confused New User").
4. **Simulated Review**: LLM adopts the persona and analyzes the input, looking for specific trigger points defined in the persona's "pet peeves."
5. **Output**: Agent appends a structured critique to a `CRITIQUE.md` file or prints to stdout.

## Tool Usage
*   **filesystem**: Read target files (`read_text_file`). Read persona definitions (JSON).
*   **shell**: Execute linters or dry-run commands to see output (which the persona might critique).
*   **web**: (Optional) Search for documentation to back up the critique.

## Memory Architecture
*   **Stateless**: Each run is independent.
*   **Configuration**: Personas are stored as JSON files in `.council/personas/`.
    *   Example: `{ "name": "Security Sam", "focus": ["auth", "input validation"], "tone": "paranoid" }`

## Failure Modes
*   **Hallucination**: Critic inventing bugs that don't exist.
*   **Tone Policing**: Critics becoming too annoying or caricature-like.
*   **Recovery**: User can adjust "temperature" or "strictness" parameters in the JSON config.

## Human Touchpoints
*   User explicitly invokes the tool.
*   User decides whether to act on the feedback.
