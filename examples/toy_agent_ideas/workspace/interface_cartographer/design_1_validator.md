# Design 1: The Manual Validator

## Purpose
To verify the accuracy of existing documentation for CLI tools and APIs by comparing stated behavior against actual execution results. This agent serves as a "Fact Checker" for technical documentation, ensuring that the "map" (docs) matches the "territory" (software).

## Core Philosophy
"Trust but Verify." The agent never assumes documentation is correct until it has witnessed the command run successfully in a controlled environment.

## Loop Structure
1.  **Ingest:** The agent reads the official documentation (man pages, web docs) for a target tool.
2.  **Parse:** It extracts listed commands, flags, and arguments into a structured "Expected Behavior" graph in Memory.
3.  **Plan:** It generates a safe execution plan (e.g., sticking to read-only commands like `list`, `get`, `describe`, `--dry-run`).
4.  **Execute:** It runs the commands via the Shell MCP.
5.  **Verify:** It compares the actual stdout/stderr/exit code with the expected result from the docs.
6.  **Report:** It flags discrepancies (e.g., "Flag `--verbose` is deprecated in v2.0 but listed in docs").

## Tool Usage
*   **web_brave_web_search:** To find and retrieve the latest online documentation.
*   **shell_execute_command:** To run the target CLI tool (e.g., `git status`, `docker ps`).
*   **memory_create_entities:** To store the "Documented Command" vs "Actual Command" entities.
*   **grep_search:** To parse complex text outputs.

## Memory Architecture
*   **Entity:** `Tool` (Name: "cURL", Version: "8.1")
*   **Entity:** `Command` (Syntax: "curl -X GET", Safety: "Safe")
*   **Relation:** `Tool` HAS `Command`
*   **Observation:** "Docs say returns JSON, but actually returns XML."

## Failure Modes
*   **Destructive Execution:** Risk of running a command that modifies state (e.g., `rm`, `drop`).
    *   *Mitigation:* Strict allowlist of "safe" verbs (list, show, dry-run). Sandbox isolation.
*   **Version Mismatch:** Docs are for v1, tool is v2.
    *   *Recovery:* Detect version string first, then search for version-specific docs.

## Human Touchpoints
*   **Initial Scope:** Human defines which tool to map and the "safety constraints" (e.g., "Only run read operations").
*   **Discrepancy Review:** Human reviews the "Errata Report" to decide if it's a bug or a doc error.
