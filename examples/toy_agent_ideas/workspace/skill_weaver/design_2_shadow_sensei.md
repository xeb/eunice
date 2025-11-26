# Design 2: The Shadow Sensei

## Purpose
A "Background Coach" that silently observes the user's coding patterns and proactively creates "Learning Quests" based on detected inefficiencies or bad habits. It solves the "Unconscious Incompetence" problem (you don't know what you don't know).

## Loop Structure
1. **Surveillance:** Agent runs as a daemon, watching file saves via `filesystem` and git commits via `shell`.
2. **Pattern Matching:** Uses `grep` and `memory` to identify "Smells" (e.g., repeating code, using slow loops instead of vectorization, old syntax).
3. **Intervention Decision:** If a pattern threshold is met, it **does not** interrupt. Instead, it creates a "Quest Card" (Markdown file) in a `.quests/` folder.
4. **Quest Injection:** The Quest contains a small, isolated refactoring exercise based on the *actual code* the user just wrote. "I noticed you wrote X. Can you rewrite it using Y? Here is a sandbox."
5. **Review:** User notices the Quest, attempts it, and the agent verifies it.

## Tool Usage
* **grep_search:** To scan the codebase for anti-patterns defined in Memory.
* **filesystem_edit_file:** To inject Quest files or comments (if permitted).
* **memory_add_observations:** To track "Habit Strength" (e.g., "User consistently avoids async/await").
* **web_brave_web_search:** To find the "Modern Way" to do what the user is doing.

## Memory Architecture
* **Entities:** `AntiPattern`, `BestPractice`, `Context` (Project A).
* **Relations:** `User --exhibits--> AntiPattern`, `BestPractice --corrects--> AntiPattern`.
* **Graph Logic:** If `exhibits` count > 5, trigger Quest.

## Failure Modes
* **Annoyance:** Too many suggestions ("Clippy" problem).
  * *Recovery:* Strict "Cool-down" timers in Memory. Only 1 quest per day.
* **Misinterpretation:** The "bad" code might be necessary for constraints the agent doesn't see.
  * *Recovery:* "Ignore this pattern" button in the Quest file.

## Human Touchpoints
* **Passive Discovery:** User finds Quests in their file tree.
* **Opt-in:** User chooses when to engage with the Quest.

## Pros/Cons
* **Pros:** High value teaching (contextual). Catches blind spots.
* **Cons:** Technically complex to parse intent. Risk of being annoying.
