# Design 3: The Deception Architect

## Purpose
To confuse and trap sophisticated attackers who might scan for "obviously fake" tokens. This design creates entire "Honeypot Projects" or "Shadow Dependencies" within the workspace that look valuable but are actually traps.

## Loop Structure
1.  **Imitate:** Analyze the user's actual projects (languages, frameworks) using `filesystem` and `grep`.
2.  **Fabricate:** Create a "Shadow Project" (e.g., `workspace/backend-api-v2`) that looks like a proprietary internal tool but is filled with tracking beacons.
3.  **Lure:** Inject references to this shadow project in ignored files or local config comments of real projects (e.g., `# TODO: Move to backend-api-v2`).
4.  **Entrap:** If a script or attacker tries to clone or scan the shadow project (directory traversal), the agent records the TTPs (Tactics, Techniques, Procedures).

## Tool Usage
*   **filesystem:**
    *   Create complex directory structures (package.json, src/, .env).
    *   Populate files with LLM-generated "proprietary code" (gibberish that looks real).
*   **memory:**
    *   Graph of "Real vs Fake" paths.
    *   Narrative of the "Deception Story" (what is this fake project supposed to be?).
*   **shell:**
    *   Use `fsnotify` to watch the fake project folder.

## Memory Architecture
*   **Entities:** `ShadowProject`, `Lure`, `AttackerProfile`.
*   **Relations:** `(Lure) POINTS_TO (ShadowProject)`, `(Attacker) ACCESSED (ShadowProject)`.

## Failure Modes
*   **User Confusion:** The user forgets which project is real and tries to work on the fake one.
    *   *Recovery:* All fake projects contain a `README.md` that is actually a warning: "THIS IS A HONEYPOT. DO NOT EDIT." visible only when opened.
*   **Disk Usage:** Generating too many fake projects fills the disk.
    *   *Recovery:* Set a quota (e.g., max 500MB of deception data).

## Human Touchpoints
*   **None (Invisible):** Ideally, the user never sees these folders unless they browse manually.
*   **Alerts:** Only when a trap is triggered.
