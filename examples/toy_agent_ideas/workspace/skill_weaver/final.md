# Agent: The Skill Weaver

## One-Line Summary
A context-aware "Personal Trainer" for developers that observes your coding patterns, detects skill gaps, and generates situated micro-exercises ("The Gym") directly within your project structure.

## Problem Domain
* **Tutorial Hell:** Developers watch videos but don't practice.
* **Context Switching:** Learning happens in a browser, coding happens in an IDE.
* **Unconscious Incompetence:** Developers don't know what they don't know (e.g., inefficient regex, ignoring new language features).

## Core Philosophy
**"Situated Learning"**: You learn best when the problem is relevant to *your* code.
Instead of a generic "Hello World" exercise, The Skill Weaver generates an exercise that says:
*"I see you used a nested for-loop in `src/main.py`. In `gym/optimization.py`, I've set up a challenge: rewrite that logic using a map/filter or list comprehension. Here is the test case."*

## Architecture

### 1. The Observer Loop (Daemon)
* **Tools:** `filesystem`, `shell` (git diff), `grep`.
* **Action:** Watches for file saves and commits.
* **Logic:** Runs lightweight static analysis (regex-based or simple AST parsing) to identify "Skill Signals".
    * *Signal:* "User pasted a 50-line function." -> *Opportunity:* "Refactoring extraction."
    * *Signal:* "User committed a 'fix typo' message." -> *Opportunity:* "Spelling/Linting tools."
    * *Signal:* "User used `time.sleep()` in a test." -> *Opportunity:* "Async/Mocking patterns."

### 2. The Knowledge Graph (Memory)
* **Tools:** `memory`.
* **Structure:**
    * **Nodes:** `Skill` (Concept), `Pattern` (Code Structure), `UserState` (Novice/Expert).
    * **Edges:** `Pattern --indicates_gap--> Skill`, `User --has_mastery--> Skill`.
* **Function:** Tracks the "Confidence Score" for each skill. If confidence is low and usage is high, it triggers an intervention.

### 3. The Dojo Generator (Intervention)
* **Tools:** `web_brave_web_search`, `filesystem_write_file`.
* **Action:** When a gap is detected (and cooldown is passed), it creates a **Micro-Dojo**.
* **Output:** A folder `./.skill_weaver/gym/context_<hash>/` containing:
    * `README.md`: "Why this matters."
    * `challenge.py`: A skeletal file importing the user's *actual code* (or a mocked version).
    * `tests.py`: A test suite the user must pass.
* **Notification:** "New Gym Challenge Available: 'Async Await Basics'. Run `weaver open` to start."

### 4. The Verification Loop
* **Tools:** `shell_execute_command`.
* **Action:** User runs `weaver check`.
* **Logic:** Runs the specific tests. If pass -> Update Memory Graph (Skill Level Up). If fail -> Provide Hints (Web Search).

## Tool Chain
* **Memory:** Stores the "Skill Tree" and "User Model".
* **Filesystem:** Reads source code, Writes Gym exercises.
* **Web:** Fetches "Best Practices" and documentation to populate the `README.md`.
* **Shell:** Runs the tests and linters.
* **Grep:** Scans for patterns to trigger lessons.

## Failure Modes & Recovery
* **Breaking the Build:** The `gym` folder is added to `.gitignore` by default so it doesn't pollute the repo.
* **Bad Exercises:** If the agent hallucinates a non-existent API, the user can flag it via `weaver flag "bad code"`. The agent uses `memory` to "forget" that pattern generation strategy.
* **Annoyance:** The agent uses a strict "Token Bucket" rate limiter in Memory. It only interrupts when it has "credits" (earned by user solving previous challenges).

## Unique Value Proposition
Unlike Copilot (which writes code for you) or LeetCode (which gives you irrelevant puzzles), **Skill Weaver** uses *your own code* as the puzzle source, turning your daily work into a continuous learning curriculum.
