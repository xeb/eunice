# Design 1: The Dependency Observer

## Purpose
The **Dependency Observer** is a passive monitoring agent designed to automate the creation of a "Technology Radar" for a software project. Instead of relying on manual updates to track which libraries and tools are in use, this agent scans the codebase to generate a real-time snapshot of the technology stack, enriching it with external data about version freshness and community health.

## Loop Structure
1.  **Scan Phase:**
    - Agent recurses through the `filesystem` to identify dependency manifests (`package.json`, `requirements.txt`, `go.mod`, `pom.xml`).
    - It parses these files to extract a list of direct and transitive dependencies.
2.  **Enrichment Phase:**
    - For each identified dependency, the agent uses `fetch` to query public registries (npm, PyPI, Maven Central) for the latest version, release date, and license.
    - It uses `web_search` to find "deprecation warnings" or "security advisories" associated with specific versions.
3.  **Reporting Phase:**
    - The agent generates a structured JSON file compatible with standard Tech Radar visualization tools (e.g., Zalando or ThoughtWorks).
    - It produces a Markdown summary: "3 Libraries Outdated", "1 Deprecated Library in Use (Moment.js)".

## Tool Usage
-   **filesystem:** Read-only access to scan directories and parse manifest files. Write access only to update the `tech-radar.json` output.
-   **fetch:** To query API endpoints of package registries (e.g., `registry.npmjs.org`).
-   **web_brave_web_search:** To find qualitative data like "is library X dead?" or "alternatives to library Y".

## Memory Architecture
-   **Stateless/File-based:** This variant relies on the filesystem for persistence. It does not maintain an internal graph. Each run is a fresh scan, ensuring the output always reflects the exact current state of the code on disk.
-   **History:** Previous reports are archived (timestamped files) to allow simple diffing by humans.

## Failure Modes
-   **Registry Downtime:** If a package registry is unreachable, the agent gracefully skips enrichment for those items, reporting "Unknown Status" rather than crashing.
-   **Parse Errors:** If a manifest file is malformed, it logs a warning and continues scanning other files.

## Human Touchpoints
-   **Consumption:** Humans read the generated reports.
-   **Configuration:** Humans can provide an `allowed_list.txt` or `ignored_paths` configuration to guide the scan.

## Pros & Cons
-   **Pros:** Safe, read-only, easy to deploy, zero side effects on code.
-   **Cons:** Reactive only; doesn't understand *how* code is used, only *that* it is installed.
