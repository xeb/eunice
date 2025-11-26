# Design 1: The Static Compliance Auditor

## Purpose
A reliable, background agent that scans software projects to generate a "Legal Bill of Materials" (SBOM + Licenses). It ensures no restricted licenses (like AGPL in proprietary code) have slipped in via transitive dependencies.

## Loop Structure
1. **Discovery:** Recursively scan directories for manifest files (`package.json`, `requirements.txt`, `pom.xml`, `go.mod`).
2. **Identification:** Extract package names and versions.
3. **Enrichment:** Query the Memory Graph for known licenses. If unknown, perform a Web Search to find the license text/type.
4. **Verification:** Compare found licenses against a "Policy Allowlist" (e.g., MIT, Apache 2.0 = OK; GPL = Flag).
5. **Reporting:** Generate a `COMPLIANCE_REPORT.md` detailing every package, its license, and the chain of custody.

## Tool Usage
- **filesystem:** Read manifest files.
- **web:** Search for "npm package [name] license" if not locally declared.
- **memory:** Store a cache of `Package@Version -> License` to avoid re-searching common libs.
- **grep:** Search for inline license headers in source files.

## Memory Architecture
- **Entities:** `Package`, `License`, `Project`.
- **Relations:** `Project DEPENDS_ON Package`, `Package HAS_LICENSE License`.
- **Logic:** Transitive reduction. If A depends on B, and B depends on C (GPL), then A is flagged.

## Failure Modes
- **Ambiguity:** Cannot determine license (reports as "UNKNOWN" for human review).
- **Dual Licensing:** Identifies multiple licenses (defaults to most restrictive).

## Human Touchpoints
- Reviewing the final Report.
- Manually overriding "UNKNOWN" classifications in a config file.
