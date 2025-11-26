# Agent: The Sovereign Sentry

## Abstract
The Sovereign Sentry is an autonomous supply chain risk agent that moves beyond simple vulnerability scanning (CVEs) to assess the **geopolitical and business risk** of software dependencies. It builds a persistent knowledge graph of maintainers, corporations, and countries to detect sanctions violations, hostile takeovers, and abandonment risks in real-time. It acts as a gatekeeper for the codebase, preventing the introduction of compromised or politically risky software while offering "safe harbor" mitigation strategies.

## Core Toolset
*   **memory**: Stores the "World Graph" (Maintainers, Companies, Countries, Sanctions, Trust Scores).
*   **web**: Performs Open Source Intelligence (OSINT) on maintainers and companies.
*   **filesystem**: Scans manifests, locks files, and manages a "quarantine" vendor directory.
*   **shell**: Enforces checks at the git/build level and runs sandboxed verification.

## Architecture

### 1. The Surveillance Loop (Background)
*   **Continuous Learning**: The agent runs a background process that queries the Web for news about top dependencies.
    *   *Search*: "Company X acquisition", "Maintainer Y sanctions", "Library Z malware".
*   **Graph Updates**:
    *   Creates entities: `Entity(Name="John Doe", Type="Maintainer")`, `Entity(Name="EvilCorp", Type="Company")`.
    *   Creates relations: `Relation(From="John Doe", Type="EMPLOYED_BY", To="EvilCorp")`.
    *   Updates observations: "Maintainer moved to sanctioned region."

### 2. The Interception Loop (Active)
*   **Trigger**: `pre-commit` hook or CI/CD pipeline step.
*   **Analysis**:
    1.  Parses `package.json` / `go.mod`.
    2.  Resolves dependency tree to leaf nodes.
    3.  Queries `memory` for the "Sovereignty Score" of each node.
*   **Decision Matrix**:
    *   **Green**: Known safe maintainer/company. Pass.
    *   **Yellow**: Unknown maintainer. Trigger "Rapid Research" (Web Search). If still unknown, Warn.
    *   **Red**: Sanctioned entity or known malicious actor. Block Build.

### 3. The Mitigation Loop (Constructive)
*   When a **Red** or **Yellow** risk is found:
    1.  **Vendor**: The agent offers to "Vendor" (copy) the specific version of the code into the repo, effectively freezing it and detaching it from the risky upstream.
    2.  **Audit**: It performs a `grep` scan of the vendored code for suspicious patterns (obfuscated strings, network calls).
    3.  **Report**: Generates a `RISK_MITIGATION.md` explaining *why* the package was vendored (e.g., "Upstream maintainer account compromised").

## Usage Example
```bash
$ git commit -m "Add cool-lib"
[TheSovereignSentry] Analyzing dependencies...
[!] CRITICAL RISK: 'cool-lib' is maintained by 'User123'.
[!] INTEL: 'User123' recently posted about selling the project to 'AdWareInc'.
[!] ACTION: Commit BLOCKED.
[!] OPTION: Run 'agent vendor cool-lib' to use a safe, frozen version.
```

## Failure Recovery
*   **False Positives**: The "World Graph" might misidentify a person.
    *   *Recovery*: A `.sentryignore` file allows manual overrides, which are logged as "Accepted Risks" in the memory graph.
*   **API Downtime**: If Web Search fails, the agent falls back to the last known state in Memory (Fail Open or Fail Closed configurable).

## Novelty
Unlike standard SCA tools (Snyk, Dependabot) which look for *bugs* (CVEs), The Sovereign Sentry looks for *bad actors* and *business risks*. It applies **Counter-Intelligence** principles to software engineering.
