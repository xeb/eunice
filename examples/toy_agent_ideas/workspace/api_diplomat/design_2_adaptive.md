# Design 2: The Adaptive Polyglot (Self-Healing SDK)

## Purpose
To act as a "Live Translator" that not only detects API changes but actively researches them and patches the local client code (SDK) to maintain connectivity. It treats API errors as learning opportunities.

## Problem Domain
When an API changes, developers waste hours reading documentation, changelogs, and StackOverflow to fix the integration. This agent automates the "Research & Fix" loop.

## Core Tools
- **web**: To search official documentation, developer portals, and changelogs when an error occurs.
- **grep**: To locate usage of the failing endpoint in the local codebase.
- **text-editor**: To modify the API client code (e.g., renaming a field, changing a type).
- **shell**: To run the project's test suite to verify the fix.

## Loop Structure
1.  **Monitor**: Listens for failed integration tests or runtime logs (via filesystem or shell output).
2.  **Diagnose**: When an API error occurs (e.g., "400 Bad Request: Field 'x' required"), it analyzes the error message.
3.  **Research**:
    - Queries the `web` for "API Provider X field x requirement change 2025".
    - Reads the provider's documentation page.
4.  **Patch**:
    - Identifies the client code responsible for the request using `grep`.
    - Uses `text-editor` to apply a fix (e.g., adding the missing field with a default value).
5.  **Verify**: Runs `shell` tests. If green, creates a Pull Request (or local patch file).

## Memory Architecture
- **Knowledge Graph**: Maps API Error Codes -> Likely Solutions.
- **Evolution History**: Tracks how an API has changed over time (e.g., "v1 -> v2 auth migration").

## Failure Modes
- **Hallucinated Fixes**: Applying a "fix" that looks syntactically correct but is semantically wrong (e.g., sending dummy data that corrupts the DB). *Mitigation*: Strict sandbox for tests; never run against production write endpoints.
- **Infinite Loops**: Repeatedly trying to patch code that keeps failing. *Mitigation*: Max retry limit per incident.

## Human Touchpoints
- **Code Review**: The agent generates a Patch/PR, but a human *must* review and merge it. The agent never commits directly to the main branch.
