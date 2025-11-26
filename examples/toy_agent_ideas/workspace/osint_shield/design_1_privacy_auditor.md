# Design 1: The Privacy Auditor (Conservative)

## Purpose
The Privacy Auditor is a defensive, local-first agent designed to sanitize your development environment and check for known credential leaks. It acts as a final gatekeeper before code is pushed and a periodic scanner for your local filesystem.

## Problem Domain
Developers often accidentally commit API keys, private keys, or PII (Personally Identifiable Information) to public repositories. Additionally, they may reuse passwords or emails that have already been compromised in data breaches.

## Core Tools
- **grep**: Used for high-speed regex matching of PII patterns (emails, IP addresses, private keys, AWS tokens) within local files.
- **filesystem**: Iterates through project directories, respecting `.gitignore` but also scanning "ignored" files that might be accidentally tracked.
- **web**: Queries public breach databases (like HaveIBeenPwned or similar APIs) and checks public git repositories.
- **text-editor**: Can auto-redact or insert "honeytokens" into files.

## Main Loop
1.  **Trigger**: Runs on a schedule (daily) or via a pre-commit hook.
2.  **Scan**:
    -   `filesystem` lists modified files.
    -   `grep` runs a battery of regex patterns (High Entropy strings, known headers like `BEGIN RSA PRIVATE KEY`).
3.  **Verify**:
    -   If a potential secret is found, the agent uses `web` to check if this specific string is already indexed in public code search engines (e.g., GitHub Code Search).
    -   It checks configured email addresses against breach databases.
4.  **Report**:
    -   Generates a `privacy_audit.md` report in the root of the workspace.
    -   Alerts the user if a *high-severity* leak is detected (e.g., active AWS key).

## Memory Architecture
- **Stateless/File-based**: This variant primarily uses the filesystem for state.
- **Allowlist**: Maintains a `.privacyignore` file to prevent false positives on known test keys.

## Failure Modes
- **False Positives**: High entropy strings (like Git commit hashes) might be mistaken for keys. *Mitigation:* Context-aware scanning (looking for "key=", "secret=").
- **API Limits**: Public search APIs may rate limit the agent. *Mitigation:* Exponential backoff.

## Human Touchpoints
- **Review**: User must manually review the `privacy_audit.md`.
- **Override**: User adds false positives to the allowlist.

## Key Insight
A "linter" for privacy that combines local static analysis with external "breach awareness".
