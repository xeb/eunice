# Design 1: The Drill Sergeant

## Purpose
A "CLI Personal Trainer" that generates, verifies, and tracks explicit coding drills upon request. It replaces static "LeetCode" sites with a dynamic local generator that runs in your actual environment.

## Loop Structure
1. **User Command:** `skill-weaver train <topic> --level <beginner|advanced>`
2. **Generation:** Agent searches web/memory for concepts, generates a drill in `./dojo/<topic>_<timestamp>/`.
3. **Artifacts:** Creates `README.md` (instructions), `exercise.py` (skeleton), and `test_exercise.py` (hidden tests).
4. **Execution:** User solves the problem in their editor.
5. **Verification:** User runs `skill-weaver check`. Agent runs the tests via `shell`.
6. **Feedback:** Agent analyzes failures, explains the concept, and updates the "Mastery Score" in Memory.

## Tool Usage
* **web_brave_web_search:** To find documentation and idiomatic examples of the requested topic.
* **filesystem:** To write the exercise files and read the user's solution.
* **shell:** To run the test suite (pytest, jest, etc.).
* **memory:** To store the user's "Skill Sheet" (Topic -> Level, History of attempts).

## Memory Architecture
* **Entities:** `Skill` (e.g., "Python List Comprehensions"), `User`, `Drill`.
* **Relations:** `User --has_mastery--> Skill`, `Drill --tests--> Skill`.
* **Observations:** "User failed drill X due to off-by-one error."

## Failure Modes
* **Test Flakiness:** Generated tests might be incorrect.
  * *Recovery:* Agent can "self-repair" the test if the user claims their solution is correct, by doing a web search for the specific error.
* **Environment Issues:** Missing dependencies.
  * *Recovery:* Agent checks `requirements.txt` before generating drills that require specific libs.

## Human Touchpoints
* **Explicit Trigger:** Nothing happens until the user asks for a drill.
* **Verification Request:** User must explicitly ask to check the solution.

## Pros/Cons
* **Pros:** Safe, contained, non-intrusive. Very clear mental model.
* **Cons:** Passive. User must know *what* they need to practice.
