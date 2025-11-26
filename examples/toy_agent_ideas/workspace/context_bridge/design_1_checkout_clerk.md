# Design 1: The Checkout Clerk

## Purpose
To enforce a structured "Shutdown Ritual" for developers, ensuring that mental context is explicitly serialized to disk before a session ends, and cleanly deserialized upon return.

## Core Loop
1.  **Trigger:** User manually runs `bridge logout`.
2.  **Interview:** Agent asks:
    *   "Current Goal?"
    *   "Next Action?"
    *   "Known Issues?"
3.  **Snapshot:** Agent records:
    *   Open files (via lsof/editors)
    *   Uncommitted git changes
    *   Current branch
    *   Timestamp
4.  **Sleep:** System saves `context/session_<timestamp>.json`.
5.  **Wake:** User runs `bridge login`.
6.  **Briefing:** Agent displays the JSON data and runs `git log --since=<timestamp>` to show what changed while the user was away.

## Tools
*   **shell:** Executing git, gathering system state.
*   **filesystem:** Reading/writing session files.
*   **text-editor:** (Optional) Opening the briefing.

## Memory Architecture
*   **Stateless:** No graph database. Relies entirely on timestamped JSON files in the `.bridge/` directory.
*   **Schema:**
    ```json
    {
      "timestamp": "2025-11-25T16:00:00Z",
      "author": "User",
      "mental_state": {
        "goal": "Fixing login bug",
        "next": "Check JWT expiry"
      },
      "system_state": {
        "branch": "fix/login",
        "dirty_files": ["src/auth.ts"]
      }
    }
    ```

## Failure Modes
*   **User Forgetting:** If user closes terminal without `bridge logout`, context is lost.
    *   *Mitigation:* Shell hooks (trap EXIT) to remind user.
*   **Corruption:** JSON file corruption.
    *   *Mitigation:* Simple text format backup.

## Human Touchpoints
*   **Explicit Entry/Exit:** User must initiate the interaction.
*   **Manual Review:** User reads the output text.
