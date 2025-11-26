# Agent: The Bureaucracy Sherpa

## Problem Domain
Modern life involves managing a complex web of administrative dependencies: visas, taxes, mortgages, insurance, and permits. Each process has strict requirements, obscure documentation, and hard deadlines. Humans struggle with the "Gap Analysis"—knowing exactly what they have versus what they need.

## Key Insight
**Requirement Graphing:** Treating bureaucratic processes not as linear to-do lists but as **Dependency Graphs** where "Target State" (e.g., Visa Approved) depends on "Satisfied Nodes" (Documents, Fees, Actions). The agent acts as a compiler that links the "External Spec" (Government Rules) to the "Internal Library" (User's Filesystem).

## Architecture

### 1. The Core Loop
1.  **Goal Ingestion:** The user creates a project folder (e.g., `projects/2025_tax_return/`) with a `goal.md` file describing the objective.
2.  **Regulatory Research (The Spec):**
    *   The agent uses `web_brave_web_search` to find official requirements.
    *   It extracts entities into the **Memory Graph**:
        *   `(Node: Requirement, Name: "W-2 Form")`
        *   `(Node: Requirement, Name: "1099-INT")`
        *   `(Edge: Tax_Return --requires--> W-2_Form)`
3.  **Asset Discovery (The Linker):**
    *   The agent scans the user's `documents/` folder using `filesystem`.
    *   It uses fuzzy matching and regex (via `grep`) to candidates for each requirement.
    *   It updates the graph: `W-2_Form --satisfied_by--> documents/finance/2024_w2_acme_corp.pdf`.
4.  **Gap Analysis & Reporting:**
    *   The agent generates a **"Readiness Report"** in the project folder.
    *   **Green:** Items found and valid.
    *   **Red:** Items missing or expired.
    *   **Yellow:** Items found but ambiguous (need human confirmation).
5.  **Acquisition Support:**
    *   For missing forms, it attempts to `fetch` the PDF from the official site.
    *   For missing data, it drafts emails to relevant parties (e.g., "Draft email to HR asking for W-2").

### 2. Memory Architecture (The Bureaucracy Graph)
The agent maintains a persistent graph in `memory`:
*   **Entities:** `Requirement`, `Document`, `Process`, `Institution`.
*   **Relations:**
    *   `Process --requires--> Requirement`
    *   `Document --satisfies--> Requirement`
    *   `Document --expires_on--> Date`
    *   `Institution --issues--> Document`

### 3. Filesystem Interface
The agent operates on a strict folder structure to maintain order:
*   `~/bureaucracy/inbox/`: Where user dumps new unorganized files.
*   `~/bureaucracy/archive/`: Where agent moves/renames files after identification.
*   `~/bureaucracy/projects/`: Active goals (Visa, Mortgage).

## Autonomy Level
**Semi-Autonomous Project Manager.**
*   **Autonomous:** Researching requirements, scanning files, linking assets, identifying gaps, drafting emails/forms.
*   **Human-Dependent:** Physically signing documents, paying fees, verifying "Yellow" matches, hitting "Send" on emails.

## Failure Modes & Recovery
*   **Misidentification:** Agent links "2023 Tax Return" to "2024 Requirement".
    *   *Recovery:* User manually edits the `status_report.md` or moves the correct file. Agent re-scans.
*   **Regulatory Hallucination:** Agent invents a requirement.
    *   *Recovery:* Agent must cite the Source URL for every Requirement node. User can click to verify.

## Example Scenario: "The Mortgage Application"
1.  **User:** "I need a mortgage pre-approval."
2.  **Agent:** Searches "Mortgage requirements [User's Bank]". Finds: Paystubs, Bank Statements, ID.
3.  **Agent:** Scans `archive/`. Finds ID and Bank Statements. Missing Paystubs.
4.  **Agent:** Writes `projects/mortgage/report.md`:
    *   [x] ID (found: `archive/id/passport.jpg`)
    *   [ ] Paystubs (last 3 months) - **MISSING**
    *   Action: Drafted email to `payroll@company.com` in `drafts/email_payroll.txt`.
5.  **User:** Sends email, gets PDF, drops in `inbox/`.
6.  **Agent:** Detects new file, moves to `archive/financial/`, updates Graph, marks Mortgage Project as "Ready".
