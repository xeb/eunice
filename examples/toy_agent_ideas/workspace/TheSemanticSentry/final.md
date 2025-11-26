# Agent Design: The Semantic Sentry

**One-Line Summary:** An autonomous log analysis daemon that uses Web Search to dynamically filter "noise" from "signal" by cross-referencing error logs with online developer discussions.

## Core Toolset
- **memory:** Stores the "Knowledge Graph" of log signatures and their derived meanings.
- **web_search:** The "Research Engine" that finds external context for unknown errors.
- **grep / shell:** The "Sensors" for reading logs and system state.

## Problem Domain
Modern systems generate gigabytes of logs. Most "Errors" are benign (deprecation warnings, known bugs, transient network glitches). Statistical anomaly detection fails because it lacks *semantic* understanding (e.g., "Error 500" is bad, but "Error 500 during shutdown" might be fine). Sysadmins suffer from alert fatigue and ignore real issues.

## Key Insight: "External Context Verification"
Instead of asking "Is this log pattern rare?", this agent asks **"What does the internet say about this log pattern?"**.
It treats the global developer community (StackOverflow, GitHub Issues, Documentation) as an extension of its knowledge base. If the top 5 search results for an error message allow it to classify the error as "Benign", it autonomously suppresses the alert.

## Architecture & Loop

### 1. Ingestion & Fingerprinting (Sensors)
- **Action:** Tails `/var/log/*` or listens on a syslog port.
- **Logic:** Regex-strips timestamps, PIDs, and variable data (IPs) to generate a stable `LogSignature`.
- **Tool:** `grep`, `shell`

### 2. Knowledge Lookup (Memory)
- **Action:** Queries the Memory Graph for the `LogSignature`.
- **Logic:** 
  - If found && status == "Benign" -> Suppress.
  - If found && status == "Critical" -> Alert immediately.
  - If not found -> Trigger **Research Mode**.

### 3. Research Mode (The Brain)
- **Action:** Uses `web_search` with the raw error string + system context (e.g., "nginx 'connection refused' upstream").
- **Logic:** 
  - Parses snippets for sentiment words ("bug", "ignore", "fixed", "harmless", "critical", "security").
  - Identifies "Consensus" (e.g., 3/5 results say "ignore").
- **Tool:** `web_search`, `web_brave_summarizer`

### 4. Judgment & Persistence (Memory)
- **Action:** Creates a new entity in the Memory Graph.
  - `Entity: LogSignature { pattern: "...", noise_score: 0.9, evidence: "http://..." }`
- **Logic:** Stores the *reason* for the judgment (the Evidence URL).

### 5. Reporting (Output)
- **Action:** 
  - **Daily Digest:** "I suppressed 400 instances of 'Error X' because [URL] says it's a known bug."
  - **Real-Time Alert:** "New CRITICAL error detected. No online mention of this specific crash. Investigate immediately."

## Memory Graph Schema
- **Entities:** `LogPattern`, `SoftwareComponent`, `WebResource`.
- **Relations:** 
  - `(LogPattern) -> GENERATED_BY -> (SoftwareComponent)`
  - `(LogPattern) -> HAS_VERDICT -> (Verdict {type: "Benign", confidence: 0.8})`
  - `(Verdict) -> CITES_EVIDENCE -> (WebResource)`

## Safety & Failure Modes
- **The "Echo Chamber" Risk:** If the web search finds a wrong answer (e.g., a novice saying "just ignore it"), the agent might suppress a real fire.
  - **Mitigation:** "Confidence Thresholds". Only suppress if multiple high-reputation domains (docs.microsoft.com, github.com/official-repo) agree.
- **Loop Prevention:** If the agent itself logs errors, it must not loop on its own logs.

## Novelty
Unlike `fail2ban` (which is rule-based) or `Splunk AI` (which is statistical), **The Semantic Sentry** is *semantic and open-ended*. It brings the "Googling the error" workflow into the automated loop.
