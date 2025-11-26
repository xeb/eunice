# Agent: The Drift Gardener

## Core Concept
**"The Drift Gardener"** is a background agent that treats Infrastructure-as-Code (IaC) drift not as an error to be suppressed, but as a signal to be managed. It operates on the philosophy of **"Reverse-IaC"**—using the live environment as the source of truth to update the code, rather than just forcing the code onto the environment.

## Problem Domain
*   **Infrastructure Drift:** "Zombie" resources that cost money but aren't in Terraform.
*   **Emergency Fixes:** Manual "ClickOps" changes made during outages that are never backported to code.
*   **Cloud Sprawl:** Difficulty tracking resources across multiple regions and accounts.

## Core Toolset
1.  **Shell:** Execute `terraform plan`, `driftctl`, `aws/gcloud` CLI.
2.  **Memory:** Stores a persistent Graph of `Resource`, `State`, and `DriftHistory`.
3.  **Filesystem:** Reads `.tf` files, writes `drift_patches/` and reports.
4.  **Grep:** Locates resource definitions to apply surgical patches.

## Execution Loop
1.  **Harvest (Discovery):**
    *   Runs `driftctl scan` or `terraform plan` to find the delta between `Live State` and `Code State`.
    *   Ingests these deltas into the **Memory Graph**.
    *   *Nodes:* `DriftItem(id="i-123", type="aws_instance", status="unmanaged")`.

2.  **Triage (Classification):**
    *   Queries the Graph to see if this drift is new or persistent.
    *   Checks against a `policy.md` (e.g., "Ignore `test-*` resources").
    *   Classifies drift as:
        *   🌱 **New Growth:** Valid new resources (e.g., a new S3 bucket for a campaign).
        *   🌾 **Mutation:** Modified attributes of existing resources.
        *   🍄 **Weeds:** Costly, untagged, or policy-violating resources (Zombies).

3.  **Cultivate (Action):**
    *   **For New Growth:** Uses `terraformer` logic to generate HCL code and places it in a `drift_patches/imports.tf` file.
    *   **For Mutation:** Generates a `.diff` patch to update the local `.tf` file to match reality.
    *   **For Weeds:** Tags the resource in the cloud with `DriftGardener:ReviewNeeded`.

4.  **Report (Visualization):**
    *   Updates `workspace/drift_report.md` with a summary of costs and coverage.
    *   If patches were generated, creates a Pull Request (via `gh` CLI if available) or a local branch.

## Persistence Strategy
*   **Memory Graph:** Holds the *history* of drift. (e.g., "This instance has been unmanaged for 30 days").
*   **Filesystem:** Holds the *artifacts* of reconciliation (patches, reports).

## Key Insight
**"Git Merge for Reality"**. Most IaC tools work like `git push --force` (overwriting reality). The Drift Gardener works like `git fetch` + `git merge`, allowing you to pull changes *from* reality back into your code.

## Failure Modes & Recovery
*   **Import Conflicts:** Generated Terraform code conflicts with existing names.
    *   *Recovery:* Agent namespaces imports with `imported_<id>` to ensure uniqueness.
*   **State Locking:** Terraform state is locked by another process.
    *   *Recovery:* Agent waits and retries; does not force unlock.
*   **API Rate Limits:** Cloud provider blocks excessive scanning.
    *   *Recovery:* Smart scheduling (scans different regions/services in rotation).

## Autonomy Level
**Checkpoint-Based.** The agent is autonomous in *reading* the world and *writing* code patches, but it **never** executes `terraform apply` or `git push` without human invocation. It tees up the work for a one-click approval.
