# Agent Design: The Context Bridge

## Executive Summary
**The Context Bridge** is an active "Session Manager" agent that reduces the cognitive load of context switching. Unlike passive tools that just save open tabs, The Context Bridge acts as a **Cognitive Checkpoint**, interviewing the developer before they leave to capture *intent* and *mental state*, and then providing a synthesized "Resumption Briefing" when they return.

## Core Philosophy
"You can't `git stash` your brain. The Context Bridge ensures your mental RAM is serialized to disk."

## Architectural Components

### 1. The Watcher (Background Daemon)
*   **Tools:** `shell`, `grep`, `filesystem`
*   **Function:** Continuously monitors the "World State" to build a *Draft Context*.
*   **Monitored Signals:**
    *   **Git Status:** Modified files, current branch, uncommitted diffs.
    *   **Shell History:** Last 50 commands (identifies build failures, test runs).
    *   **Editor State:** (Optional) List of open files via `lsof` or editor API.
*   **Output:** A volatile in-memory object:
    ```json
    { "focus": "auth_middleware", "status": "failing_tests", "files": ["src/auth.ts"] }
    ```

### 2. The Interceptor (Exit Trigger)
*   **Tools:** `shell` (hooks)
*   **Function:** Detects end-of-session intent (typing `exit`, closing window, or idle timeout).
*   **Interaction:**
    *   **Proactive:** "Wait! You have 3 uncommitted files and the last test failed."
    *   **The Bridge Protocol:** A 30-second text interview.
        1.  *Agent:* "It looks like you were fixing the JWT bug. Is that correct?" (Derived from Watcher)
        2.  *User:* "Yes."
        3.  *Agent:* "What is the very next thing you need to do when you return?"
        4.  *User:* "Check the expiration timestamp logic."
    *   **Synthesis:** Saves a `.bridge/latest.md` file.

### 3. The Sentinel (Gap Monitoring)
*   **Tools:** `fetch`, `shell`
*   **Function:** While the user is away, it periodically (e.g., hourly) runs `git fetch` and checks for activity in the repo (PRs, commits by others) that touch the files in the `.bridge/latest.md`.
*   **Key Insight:** It only reports changes *relevant* to the user's suspended context.

### 4. The Primer (Entry Briefing)
*   **Tools:** `shell`, `filesystem`
*   **Function:** On the next shell startup:
    1.  Reads `.bridge/latest.md`.
    2.  Checks Sentinel logs for external changes.
    3.  **Displays The Bridge:**
        > **Welcome Back.**
        > *Last Session:* Fixing JWT bug.
        > *Next Action:* Check expiration timestamp logic.
        > *Alert:* Alice merged a change to `src/auth.ts` 2 hours ago. You should pull before continuing.
    4.  **Restoration:** Offers to re-open the files: `open src/auth.ts`.

## Loop Structure
1.  **Session Start:** Display Briefing -> User Confirms -> Start Watcher.
2.  **Working:** Watcher tracks state in background.
3.  **Session End:** Interceptor triggers Interview -> Saves Bridge File -> Stops Watcher -> Starts Sentinel.

## Tool Usage
*   **memory:** Stores the "User Persona" (typical working hours, preferred editor, recurring projects).
*   **filesystem:** Stores the Bridge files (Markdown) and Sentinel logs.
*   **shell:** Executes git commands, monitors processes, and handles user input.
*   **grep:** Analyzes code diffs during the Gap Monitoring phase.

## Failure Modes & Recovery
*   **Forced Exit (Crash/Power Loss):** The Interceptor never runs.
    *   *Recovery:* On startup, the agent detects a "stale lockfile" implies a crash. It falls back to the *last known state* from the Watcher (which checkpoints every 5 mins) and asks: "It looks like we crashed. Were you working on X?"
*   **False Positives:** Sentinel warns about irrelevant changes.
    *   *Mitigation:* The user can mark files as "High Priority" in the exit interview to filter noise.

## Human Touchpoints
*   **The Exit Interview:** The defining feature. It forces a moment of reflection, converting "implicit context" into "explicit documentation."
*   **The Entry Briefing:** A "Daily Standup" with yourself.

