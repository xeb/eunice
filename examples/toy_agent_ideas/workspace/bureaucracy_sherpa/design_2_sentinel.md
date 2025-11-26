# Design 2: The Compliance Sentinel

## Purpose
A "Digital Twin" of the user's legal and bureaucratic existence. It runs in the background, monitoring the expiration dates of passports, leases, insurance policies, and subscriptions, and proactively researching renewal requirements before they become emergencies.

## Core Loop
1. **Ingest:** Agent recursively scans a protected `documents/` folder using `filesystem`.
2. **extract:** It uses OCR (via shell tools or simulated text extraction) to find "Entities" (Passport, Lease) and "Properties" (ExpiryDate, ID Number).
3. **Graph Update:** It stores these in the `memory` graph:
   - `[User] --has_document--> [Passport] --expires--> [2026-05-20]`
4. **Monitor:** Daily, it checks the graph for upcoming expiries (e.g., < 6 months).
5. **Research:** If an expiry is approaching, it uses `web_brave_web_search` to find *current* renewal requirements (e.g., "US Passport renewal time 2025").
6. **Alert:** It creates a "Warning" file in the user's inbox with a timeline.

## Tool Usage
- **filesystem**: Scanning user documents.
- **memory**: Storing the "State of the User" (Dates, IDs).
- **web**: Checking processing times and rule changes.
- **grep**: Finding patterns like dates in text files.

## Memory Architecture
- **Graph-Heavy:** The "Truth" is in the graph. The files are just evidence.
- **Ontology:** Nodes for `Document`, `Institution`, `Deadline`, `Constraint`.

## Failure Modes
- **Privacy Leak:** Storing sensitive data in the graph. Mitigation: Store hashes or references, not PII, where possible. Or keep graph local.
- **Misinterpretation:** Confusing "Issue Date" for "Expiry Date". Mitigation: Human verification of the initial graph nodes.

## Human Touchpoints
- Initial setup (pointing to document folder).
- Verifying the extracted dates.
- Acting on alerts.

## Pros/Cons
- **Pros:** High value (prevents fines/hassle), proactive, "set and forget".
- **Cons:** High privacy risk, requires OCR/Vision capabilities (simulated here via text), complex state management.
