# Design 1: The Mirror (Observational)

## Purpose
"The Mirror" acts as a passive observer that maintains a high-fidelity "Digital Twin" of the infrastructure in the Memory Graph. Its primary goal is **Observability**—making the invisible "Dark Matter" of unmanaged cloud resources visible and queryable without taking dangerous actions.

## Loop Structure
1.  **Discovery:** Periodically runs shell commands (`aws ec2 describe-instances`, `gcloud compute instances list`) or dedicated tools (`driftctl`).
2.  **Ingestion:** Parses the JSON output and creates entities in the **Memory Graph**.
    *   Nodes: `EC2Instance`, `S3Bucket`, `SecurityGroup`.
    *   Edges: `USES`, `ATTACHED_TO`, `ALLOWS_TRAFFIC_FROM`.
3.  **Mapping:** Scans local `.tf` files using `grep` to find corresponding resource definitions.
    *   If found: specifices relation `MANAGED_BY -> File("main.tf")`.
    *   If not found: marks entity as `UNMANAGED` or `ZOMBIE`.
4.  **Reporting:** Generates a `drift_report.md` in the filesystem, categorizing resources by their management status and cost implication.

## Tool Usage
*   **Shell:** Execute read-only cloud CLI commands.
*   **Memory:** Store the topology of the infrastructure.
    *   *Why?* Allows for complex queries like "Show me all unmanaged security groups allowing port 22".
*   **Filesystem:** Write reports; Read `.tf` files.
*   **Grep:** Fast search for resource IDs in local codebase.

## Memory Architecture
*   **Entity Types:** `Resource`, `IaC_File`, `Cost_Center`, `Alert`.
*   **Relations:**
    *   `Resource` -> `IaC_File` (MANAGED_BY)
    *   `Resource` -> `Resource` (DEPENDS_ON)
    *   `Resource` -> `Observation` (HAS_STATE)

## Failure Modes
*   **API Throttling:** If it queries too fast, cloud provider blocks it.
    *   *Recovery:* Exponential backoff implemented in the shell script wrapper.
*   **Stale Data:** Graph desynchronizes from reality.
    *   *Recovery:* "Full Refresh" cycle every 24 hours that wipes and rebuilds the graph.

## Human Touchpoints
*   **Read-Only:** The user only interacts by reading the generated Markdown reports.
*   **Query:** User can use the Memory tool to ask questions about the infrastructure.
