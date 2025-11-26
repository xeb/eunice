# Design 2: The Reconciler (Reverse-IaC)

## Purpose
"The Reconciler" takes the philosophy that "Reality is Truth." If the infrastructure has drifted from the code, the agent assumes the infrastructure is correct (perhaps an emergency hotfix was applied) and attempts to update the codebase to match. It automates the painful process of importing existing resources into Terraform/Pulumi.

## Loop Structure
1.  **Drift Detection:** Runs `terraform plan -detailed-exitcode`.
    *   Exit code 2 means drift detected.
2.  **Analysis:** Parses the diff. Identifies resources that exist in cloud but not in state (New/Zombie) or resources that have changed attributes (Drift).
3.  **Code Generation:**
    *   **For New Resources:** Uses `terraformer` or custom logic to generate HCL code blocks for the found resources. Places them in `drift_patches/imports.tf`.
    *   **For Modified Attributes:** Generates a patch to update the existing resource definition (e.g., changing `instance_type` from `t2.micro` to `t3.medium`).
4.  **Verification:** Runs `terraform plan` with the new patches applied to a shadow workspace.
5.  **Proposal:** Commits the working patches to a new git branch `drift-fix-<timestamp>` and opens a PR (or creates a `patch_proposal.md` if no git).

## Tool Usage
*   **Shell:** `terraform`, `terraformer`, `git`.
*   **Filesystem:** Writing `.tf` files, creating patch directories.
*   **Grep:** Locating where resources are defined to apply attribute updates.
*   **Memory:** Tracks "Ignored" resources that the user has explicitly rejected importing.

## Memory Architecture
*   **Nodes:** `DriftEvent`, `PatchProposal`.
*   **Edges:** `DriftEvent` -> `PatchProposal` (RESOLVED_BY).
*   **Observations:** Store the raw diff output and the validation result of the generated code.

## Failure Modes
*   **Destructive Imports:** Importing a resource might trigger a recreation if not done perfectly.
    *   *Mitigation:* The agent *never* runs `apply`. It only generates code and verifies with `plan`.
*   **Secrets Exposure:** Importing resources might pull sensitive data into plain text state.
    *   *Mitigation:* Grep filters to redact known secret patterns (API keys, passwords) before writing files.

## Human Touchpoints
*   **Merge Review:** The human must review the generated branch/PR. The agent does the heavy lifting of writing the code, but the human approves the intent.
