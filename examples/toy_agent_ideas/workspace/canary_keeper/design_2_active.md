# Design 2: The Active Sentinel

## Purpose
A proactive defense agent that wraps package manager commands (npm, pip, cargo) to monitor their behavior in real-time. Instead of just planting files, it watches *who* accesses them using system tracing tools.

## Loop Structure
1.  **Intercept:** The agent provides wrapper aliases (e.g., `safe-npm`) or uses shell hooks to intercept install commands.
2.  **Trace:** Executes the installation command inside a supervised environment (using `strace`, `auditd`, or `bwrap`).
3.  **Analyze:** Real-time stream analysis of system calls:
    *   **Network:** Is it connecting to known bad IPs? (Web Search check)
    *   **Files:** Is it trying to read `~/.ssh/id_rsa` or `/etc/passwd`?
    *   **Processes:** Is it spawning unexpected sub-shells?
4.  **Block:** If a violation is detected, send `SIGKILL` to the process tree and rollback changes.

## Tool Usage
*   **shell:**
    *   `execute_command`: To run the trace tools (`strace -e trace=open,connect ...`).
    *   To kill malicious processes.
*   **memory:**
    *   Store "Behavioral Profiles" of known good packages.
    *   Whitelist of allowed paths (e.g., `./node_modules` is allowed, `~/.ssh` is forbidden).
*   **web:**
    *   `brave_web_search`: To check reputation of domains contacted during install.

## Memory Architecture
*   **Entities:** `Package`, `SystemCallPattern`, `NetworkDestination`.
*   **Relations:** `(Package) EXHIBITS (SystemCallPattern)`, `(Package) CONTACTS (NetworkDestination)`.

## Failure Modes
*   **Performance:** Tracing slows down installation significantly.
    *   *Recovery:* Allow "Sampling Mode" (check only new packages) or "Post-Mortem Mode" (audit after install).
*   **False Positives:** Complex build scripts (e.g., node-gyp) often look like malware (accessing system headers).
    *   *Recovery:* User prompting "Allow this behavior once/always?".

## Human Touchpoints
*   **Authorization:** "Package 'evil-lib' is trying to access ~/.ssh. Block or Allow?"
*   **Review:** Weekly report of "suspicious behaviors" blocked or allowed.
