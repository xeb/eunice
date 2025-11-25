# Design 3: The Chaos User

## Purpose
To test the resilience and "helpfulness" of the software by intentionally behaving like a "bad" or "clumsy" user (typos, wrong order, missing args) and grading the software's response.

## Core Loop
1. **Fuzzing Strategy:**
   - specific targets: Typos (`inits` instead of `init`), Argument Swaps (`cp dest src`), Missing Deps.
2. **Execution:** Run the "bad" command via `shell`.
3. **Sentiment Analysis (The "Grumpiness" Factor):**
   - Check if the error message is:
     - **Helpful:** "Unknown command 'inits', did you mean 'init'?" (+1 Score)
     - **Neutral:** "Command not found." (0 Score)
     - **Hostile/Crash:** "Segmentation fault" or Python stack trace. (-5 Score)
4. **Resilience Check:** Does the application leave the filesystem in a corrupted state after a bad command? (Uses `filesystem` to diff state).

## Tool Usage
- **shell:** Inject faults/commands.
- **filesystem:** Verify state consistency (pre/post snapshot).
- **memory:** Track "Fragile Areas" of the application.

## Memory Architecture
- **Map of Weaknesses:** Stores specific command chains that trigger unhelpful errors.

## Failure Modes
- **System Corruption:** A "clumsy" user might actually break the test environment.
- **Subjectivity:** Determining if an error is "helpful" is hard without an LLM classifier.

## Human Touchpoints
- **Review:** Humans review the "Most Frustrating Moments" log.
