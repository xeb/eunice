# Design 3: The Socratic Drill Sergeant (Experimental)

## Purpose
To prevent "Passive Consumption" (watching tutorials without learning). This agent refuses to advance the curriculum until the user demonstrates understanding via active recall.

## Loop Structure
1. **Study:** User reads material in `Current_Module/`.
2. **Challenge:** User runs `autodidact verify`.
3. **Quiz Gen:** Agent generates a quiz based on the linked resources using LLM (simulated or remote) + `memory` of key terms.
4. **Grading:** User answers in `quiz_answers.md`. Agent checks answers.
5. **Progression:**
   - **Pass:** Archive `Current_Module` to `Archive/`, generate next module.
   - **Fail:** Find *different* resources (explain it another way) and append to `Current_Module/`.

## Tool Usage
- **text-editor:** Agent reads user's notes/answers to assess depth.
- **web:** Agent searches for *specific* questions/exercises (e.g., "Linear Algebra practice problems").
- **memory:** Tracks "Spaced Repetition" intervals for past concepts.

## Memory Architecture
- **Mastery State:** Nodes track a "Stability Score" (how well known it is) and "Last Reviewed" timestamp.
- **Resurfacing:** Agent re-injects old questions into new module quizzes.

## Failure Modes
- **Frustration:** User gets stuck and quits. (Recovery: "Cheat Mode" or "Override").
- **Bad Grading:** Agent misinterprets a correct answer. (Recovery: User disputes grade).

## Human Touchpoints
- **The Exam:** Explicit interaction required to progress.
- **Dispute:** User arguing they understood the concept despite the wrong answer.
