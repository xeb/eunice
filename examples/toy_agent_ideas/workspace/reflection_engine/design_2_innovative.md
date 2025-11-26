# Design 2: The Shadow Mentor

## Purpose
A real-time "sidecar" agent that watches the developer's work in progress and offers preventative advice based on a deep Memory Graph of their psychological and technical tendencies.

## Loop Structure
1. **Observation:** The agent runs as a daemon, using `filesystem` (inotify) to watch for file changes.
2. **Contextualization:**
   - When `api.ts` is modified, it queries the **Memory Graph**:
     - "What errors has the user made in `api.ts` before?"
     - "What concepts related to the imported modules (e.g., `axios`, `auth`) have associated failure patterns?"
3. **Inference:**
   - It doesn't just look for regex matches. It looks for *absence*.
   - "User added a new async function but didn't add a try/catch block. In the past, 80% of their bugs were unhandled promise rejections."
4. **Intervention:**
   - It writes a non-intrusive comment to a `SHADOW_MENTOR.md` file open in a split pane, or decorates the line (if IDE integration allows).
   - "⚠️ You are using `fs.readFileSync`. You reverted this 2 weeks ago in favor of `fs.promises.readFile`. Recall: 'Blocking I/O froze the UI'."

## Tool Usage
- **memory:** Stores entities (Files, Functions, Libraries, Concepts) and observations (ErrorTypes, FixPatterns).
- **filesystem:** Watches files, reads content.
- **grep:** Scans for structural patterns.
- **web:** Fetches docs if the error implies a misunderstanding of a library.

## Memory Architecture
- **Graph Database:**
  - **Nodes:** `User`, `File`, `Commit`, `ErrorPattern`, `Concept` (e.g., "Async", "Memory Management").
  - **Edges:** `User --STRUGGLES_WITH--> Concept`, `Commit --FIXED--> ErrorPattern`.
- **Reasoning:** "If User struggles with Regex, and File contains Regex, Increase Alert Level."

## Failure Modes
- **Annoyance:** Too many warnings break flow.
  - *Recovery:* The agent tracks "Accepted" vs "Ignored" advice. It creates a feedback loop to lower its own confidence threshold if ignored often.
- **Resource Usage:** Constant parsing of files.
  - *Mitigation:* Debounce analysis (e.g., only run 5s after user stops typing).

## Human Touchpoints
- **Implicit Feedback:** Ignoring the advice is a signal.
- **Explicit Training:** User can tell the agent "This wasn't a mistake, it was a hack." The agent stores this as an exception.
