# Design 1: The Contextual Prompt Bot

## Purpose
To break creative blocks by injecting random but grammatically relevant questions into the user's workflow. It forces the user to justify their decisions against arbitrary constraints or metaphors.

## Loop Structure
1. **Watch**: Monitor `*.md` or `*.txt` files in `workspace/` for modification.
2. **Contextualize**: When a file changes, read the last paragraph to get the current "Topic".
3. **Fetch**:
   - Pick a random "Oblique Strategy" or "Random Wikipedia Title" via `web_brave_web_search`.
   - OR use a built-in list of "Universal Questions" (e.g., "What would this look like if it were 10x smaller?").
4. **Prompt**: Append a question to `workspace/prompts.log` or a specific `_inspiration.md` sidecar file.
   - *Example:* "Topic: API Design. Random Concept: Baroque Architecture. Prompt: How can this API be more ornate yet structurally sound?"

## Tool Usage
- **filesystem**: Read `last_modified` files. Write to `prompts.log`.
- **web**: `brave_web_search` for "random wikipedia article" or specific creative prompt databases.
- **shell**: `tail` to get recent context.

## Memory Architecture
- **Stateless**: Does not remember past prompts or user context deeply.
- **Log-based**: Keeps a simple append-only log of prompts generated.

## Failure Modes
- **Distraction**: User finds the prompts annoying. (Mitigation: Low frequency, e.g., once per hour).
- **Irrelevance**: Prompts are too random. (Mitigation: Use `web` to find *slightly* related topics).

## Human Touchpoints
- **Passive**: User reads the log when they are stuck.
- **Active**: User can run `trigger_prompt` to get an immediate idea.
