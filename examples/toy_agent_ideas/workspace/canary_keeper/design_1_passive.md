# Design 1: The Passive Minefield

## Purpose
To detect unauthorized file access and data exfiltration in local development environments by planting "Canary Tokens" (fake secrets) and monitoring their integrity. This agent acts as a tripwire for supply chain attacks (e.g., malicious npm packages) that attempt to harvest credentials.

## Loop Structure
1.  **Survey:** Scan the project structure to identify "attractive" locations for secrets (e.g., `.env`, `config/secrets.yaml`, `.aws/credentials`).
2.  **Plant:** Generate realistic-looking but fake credentials (honeytokens) and place them in these locations.
3.  **Monitor:**
    *   **Local Check:** Periodically check file access timestamps (atime) for these tokens.
    *   **Remote Check:** If using web-beacons (DNS/HTTP tokens), listen for alerts from the token provider (via `fetch`).
4.  **Report:** If a token is touched by an unknown process, log the incident and alert the user.

## Tool Usage
*   **filesystem:**
    *   `list_directory`: To find where to plant tokens.
    *   `write_file`: To create the fake credentials.
    *   `get_file_info`: To check `atime` (access time).
*   **memory:**
    *   Store the mapping of `TokenID` -> `FilePath`.
    *   Store the "State" of each token (Planted, Triggered, Missing).
*   **web:**
    *   `fetch`: To register tokens with a service like Canarytokens.org (optional) or to check threat intel on suspicious IPs.

## Memory Architecture
*   **Entities:** `CanaryToken`, `FileLocation`, `Incident`.
*   **Relations:** `(CanaryToken) LOCATED_AT (FileLocation)`, `(Incident) TRIGGERED_BY (CanaryToken)`.

## Failure Modes
*   **False Positives:** The user's own editor or grep tool reads the file.
    *   *Recovery:* The agent maintains a "Allowlist" of processes (e.g., `code`, `git`, `grep`) and ignores reads from them if possible, or asks the user to confirm "Was this you?".
*   **Cleanup Failure:** Leaving fake secrets behind when the agent is uninstalled.
    *   *Recovery:* A "Manifest" is kept in `workspace/canary_keeper/manifest.json` to allow manual cleanup.

## Human Touchpoints
*   **Incident Alert:** "Warning: The fake AWS key in .env was read by 'node'. Did you run a script?"
*   **Configuration:** User defines which directories to protect.
