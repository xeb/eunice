# Agent: The Canary Keeper

## Purpose
A multi-layered defense agent for local development environments that protects against software supply chain attacks (e.g., malicious npm/pip packages) and unauthorized data access. It combines "Active Deception" (planting honeytokens) with "Proactive Surveillance" (process monitoring) to detect when untrusted scripts try to steal credentials or data.

## Core Toolset
*   **filesystem:** To plant decoy files, scan for high-value targets, and manage logs.
*   **shell:** To execute system tracing (`strace`, `auditd`, `fsnotify`) and kill malicious processes.
*   **memory:** To maintain a "Trust Graph" of allowed processes/paths and a ledger of planted tokens.
*   **web:** To fetch threat intelligence on suspicious IPs and integrate with external canary services (e.g., Thinkst Canary).

## Architectural Logic (The "Defcon" Loop)

The agent operates in three modes, adjustable by the user or triggered by threat level:

### Mode 1: The Minefield (Low Overhead)
*   **Action:** Plants fake `.env`, `id_rsa`, and `aws_credentials` files in the project directory (gitignored).
*   **Surveillance:** Periodically checks file access times (`atime`).
*   **Trigger:** If a token is read by anything other than the user's whitelist (e.g., VS Code), it logs an incident.

### Mode 2: The Sentinel (Active Install Monitoring)
*   **Action:** Wraps `npm install`, `pip install`, etc.
*   **Surveillance:** Uses `strace` or `bwrap` to sandbox the installation process.
*   **Trigger:**
    *   **Network:** Connects to non-registry IP? -> **BLOCK**.
    *   **Filesystem:** Reads `~/.ssh` or `/etc/shadow`? -> **BLOCK**.
    *   **Process:** Spawns `/bin/sh -i`? -> **BLOCK**.

### Mode 3: The Labyrinth (High Deception)
*   **Action:** Creates entire fake "proprietary" sub-projects with realistic names (e.g., `auth-service-v2`).
*   **Surveillance:** Watches for directory traversal attempts.
*   **Trigger:** Any access to these folders by a non-human user is treated as a critical breach.

## Memory Graph Structure
The agent persists its state in the Memory MCP:

*   **Nodes:**
    *   `CanaryToken` (path, type, creation_time)
    *   `TrustedProcess` (name, hash, parent_pid)
    *   `Incident` (timestamp, culprit_pid, action)
*   **Edges:**
    *   `(TrustedProcess) ALLOWED_TO_READ (CanaryToken)`
    *   `(Incident) TRIGGERED_BY (CanaryToken)`

## Failure Modes & Recovery
1.  **False Alarm Storm:** A backup script reads all files, triggering all canaries.
    *   *Recovery:* Agent detects "Mass Access" pattern and temporarily silences alerts, asking user "Is process 'backup-daemon' trusted?".
2.  **Zombie Tokens:** User deletes the agent but tokens remain.
    *   *Recovery:* Agent leaves a `canary_manifest.json` in the workspace root. A simple shell script can read this and `rm` all tokens.
3.  **Performance Drag:** `strace` makes installs slow.
    *   *Recovery:* Defaults to "Light Mode" (monitoring only file access via `fsnotify`) unless a "High Risk" package (low reputation) is detected.

## Practical Usage
**Example Scenario:**
1.  User runs `canary-keeper plant`.
2.  Agent adds a fake `AWS_SECRET_ACCESS_KEY` to `.env`.
3.  User runs `npm install malicious-lib`.
4.  `malicious-lib`'s post-install script greps for "AWS".
5.  Agent detects the read on the fake token.
6.  Agent kills the process, deletes `node_modules`, and alerts: "Blocked malicious-lib from stealing AWS keys."
