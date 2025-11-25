# Design 3: The Protocol Translator

## Purpose
To create a "Rosetta Stone" for technical tools. This agent takes a known command in Tool A (e.g., `docker ps`) and finds the equivalent in Tool B (e.g., `podman ps`, `kubectl get pods`).

## Core Philosophy
"Equivalence through Output." If two commands produce the same semantic information (even if formatted differently), they are equivalent.

## Loop Structure
1.  **Baseline Execution:** Run the "Source Command" (Tool A) and capture the output (the "Ground Truth").
2.  **Semantic Parsing:** Use the LLM to understand the *meaning* of the output (e.g., "List of 3 running containers").
3.  **Search & Hypothesize:** Search docs for Tool B to find similar commands.
4.  **Target Execution:** Run candidate commands in Tool B.
5.  **Equivalence Check:**
    *   Compare the semantic payload of Output B with Output A.
    *   If they match, record a "Translation Pair".
6.  **Code Generation:** Generate a wrapper script or alias file that makes Tool B behave like Tool A.

## Tool Usage
*   **web_brave_web_search:** To find "Tool A vs Tool B" cheat sheets to seed the search.
*   **shell_execute_command:** To run both tools simultaneously.
*   **memory_create_relations:** To link `Command_A` --(EQUIVALENT_TO)--> `Command_B`.
*   **text-editor_edit_text_file_contents:** To write the final "Migration Guide" or "Wrapper Script".

## Memory Architecture
*   **Entity:** `Concept` (e.g., "List Processes")
*   **Relation:** `Command_A` IMPLEMENTS `Concept`
*   **Relation:** `Command_B` IMPLEMENTS `Concept`
*   **Result:** `Command_A` == `Command_B` (Confidence: 95%)

## Failure Modes
*   **False Equivalence:** `rm -rf` looks like `trash-put` but one is permanent and one is not.
    *   *Mitigation:* Check for "destructive" flags in documentation before testing equivalence.
*   **Context Dependency:** Tool B requires a daemon, Tool A does not.
    *   *Recovery:* Agent notes "Pre-requisites" in the memory graph.

## Human Touchpoints
*   **Use Case Definition:** Human specifies the migration path (e.g., "I want to move from AWS CLI to Azure CLI").
*   **Validation:** Human confirms the generated aliases are useful.
