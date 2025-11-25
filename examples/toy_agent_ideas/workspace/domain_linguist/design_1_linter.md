# Design 1: The Domain Linter (Conservative)

## Purpose
To prevent "Semantic Drift" by strictly enforcing a defined "Ubiquitous Language" during the CI/CD process. It ensures that terms used in code (class names, variables) match the terms defined in the project's dictionary, acting as a gatekeeper against ambiguity.

## Core Loop
1.  **Trigger:** Runs continuously on file change or via CI hook.
2.  **Load:** Reads `domain_dictionary.yaml` (approved terms) and `domain_blacklist.yaml` (banned terms).
3.  **Scan:** Uses `grep` to find occurrences of blacklisted terms or "fuzzy" matches that deviate from the dictionary (e.g., using `client_id` when `customer_id` is the standard).
4.  **Report:** Generates a `semantic_drift_report.json` and prints violations to `stderr`.
5.  **Fail:** Returns a non-zero exit code if "High Severity" violations are found.

## Tool Usage
*   `filesystem`: Reads configuration and source code.
*   `grep`: Fast pattern matching for banned terms.
*   `shell`: Execution within CI pipelines.

## Memory Architecture
*   **Stateless:** Relies entirely on configuration files (`.yaml`) committed to the repo. It does not maintain a persistent graph between runs, ensuring deterministic behavior in CI.

## Failure Modes
*   **False Positives:** Flagging valid code (e.g., library calls) as violations.
    *   *Recovery:* Allow `// ignore-domain-check` comments or a `.domainignore` file.
*   **Performance:** Scanning large monorepos.
    *   *Recovery:* Incremental scanning (only changed files in PR).

## Human Touchpoints
*   **Configuration:** Humans must curate the `domain_dictionary.yaml`.
*   **CI Blocks:** Humans must fix naming issues to merge code.

## Pros/Cons
*   **Pros:** Safe, deterministic, easy to integrate.
*   **Cons:** Annoying "nags", doesn't help *fix* the problem, high maintenance of the YAML file.
