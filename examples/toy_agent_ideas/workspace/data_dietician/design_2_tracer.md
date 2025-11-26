# Design 2: The Cloud Tracer (Hybrid/Linker)

## Purpose
To reduce local storage usage by identifying files that are effectively "cached" from the public web. If a local PDF, code library, or image exists reliably online (arXiv, GitHub, Unsplash), the agent replaces the heavy local copy with a lightweight "Pointer" file.

## Core Loop
1. **Identify:** Finds large, non-private files (e.g., `Downloads/`, `Library/`).
2. **Fingerprint:** Generates a hash or extracts specific keywords/titles from the file.
3. **Search:** Uses `web_brave_web_search` to find an exact match online.
4. **Verify:** Checks if the URL is stable/canonical (e.g., `doi.org`, `github.com`).
5. **Propose:** Creates a `relocation_plan.md` list.
6. **Execute:** Upon approval, deletes the local file and creates `[filename].url` or `[filename].md` with the source link and metadata.

## Tool Usage
- `filesystem`: Reading file headers/content for fingerprinting.
- `web`: Searching for the file online.
- `fetch`: Verifying the remote link is alive and matches the content.
- `shell`: Deleting files and creating shortcuts.

## Memory Architecture
- **Nodes:** `File`, `ExternalSource`.
- **Edges:** `AVAILABLE_AT`, `REPLACED_BY`.
- **Persistence:** Keeps a ledger of "Dehydrated" files so they can be re-downloaded if needed.

## Failure Modes
- **Link Rot:** The external URL dies. *Mitigation:* `digital_amber` agent (companion) or only linking to DOIs/Archives.
- **False Positive:** Linking to a different version of the same file. *Mitigation:* Hash comparison where possible.

## Human Touchpoints
- **Approval:** User reviews the "Relocation Plan" before execution.
- **Restoration:** User clicks the link to retrieve the file.
