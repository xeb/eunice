# Design 3: The Diplomat (Hybrid/Constructive)

## Purpose
Instead of just blocking or reporting, The Diplomat helps *mitigate* the risk by finding alternatives or "vendoring" code safely. It acknowledges that you usually *need* the library, even if it's risky. It focuses on "Sovereignty" - ensuring you own your dependencies.

## Loop Structure
1.  **Assessment**: Identifies a high-risk dependency (e.g., abandoned, hostile nation owner).
2.  **Option Generation**:
    *   **Search**: Uses `web` to find alternative libraries with similar functionality but safer governance.
    *   **Fork**: Uses `shell` to clone the repo into a local `vendor/` directory, effectively detaching it from the upstream supply chain.
3.  **Refactoring**:
    *   If an alternative is chosen, it uses `grep` and `text-editor` to attempt a rudimentary replacement of import statements (proposing a refactor).
4.  **Verification**:
    *   Runs the project's test suite to see if the fork/replacement works.
5.  **Proposal**: Opens a Pull Request with the vendored code or the replacement, detailed with the risk assessment.

## Tool Usage
*   **filesystem**: Heavily used for copying/vendoring code (`cp -r`, `git clone`).
*   **web**: "Vs" searches (e.g., "moment.js alternatives").
*   **shell**: Running tests (`npm test`) to validate the mitigation.
*   **memory**: Tracks "Vendored" status and upstream sync needs.

## Memory Architecture
*   **Hybrid**: Uses filesystem for the code (Single Source of Truth) but uses Memory to track *why* something was vendored (the risk rationale) and when to check upstream again (e.g., "Check back in 6 months if maintainer changes").

## Failure Modes
*   **Breaking Changes**: Automatic refactoring fails (very likely).
*   **License Violation**: Forking/Vendoring might violate specific licenses if headers aren't preserved.
*   **Mitigation**: The agent includes a "Legal Check" step, parsing the `LICENSE` file before vendoring.

## Human Touchpoints
*   **Decision**: The agent prepares the "Safe Harbor" PR, but the human must merge it.
*   **Negotiation**: The agent can generate an issue template to post on the upstream repo asking for clarification on ownership.
