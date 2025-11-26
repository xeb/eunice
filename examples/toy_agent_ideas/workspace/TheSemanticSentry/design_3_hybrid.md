# Design 3: The Semantic Firewall (Hybrid)

## Purpose
An intelligent filter that sits between raw logs and the notification system. It uses web evidence to "downrank" noisy alerts, ensuring humans are only woken up for truly novel or critical issues.

## Loop Structure
1. **Stream:** Ingests logs from a pipe or file.
2. **Fingerprint:** Simplifies log lines into a signature (removing timestamps/IDs).
3. **Consult Graph:** Checks `memory` for this signature's "Noise Score".
4. **Research (Async):** If the signature is new, triggers a background `web_search`.
   - If search results indicate "harmless warning", updates Graph with High Noise Score.
   - If search results indicate "database corruption", updates Graph with Low Noise Score.
5. **Route:** 
   - High Noise -> /dev/null or daily digest.
   - Low Noise -> Critical Alert Channel.
6. **Explain:** Alerts include the *reasoning* (e.g., "Flagged as critical because search results mention 'data loss'").

## Tool Usage
- **memory:** The central "judgement database".
- **web_search:** The "Oracle" for unknown errors.
- **grep/shell:** Log ingestion.

## Memory Architecture
- **Nodes:** `LogSignature`, `ContextTag` (e.g., "DeprecationWarning", "ConnectionRefused").
- **Relations:** `(LogSignature) -> IS_RELATED_TO -> (ContextTag)`.

## Failure Modes
- **False Negatives:** Might suppress a real error because it looks like a known benign one.
- **API Rate Limits:** Too many new logs might exhaust search quotas.
- **Recovery:** Fallback to "Pass-Through" mode if the agent is overwhelmed or offline.

## Human Touchpoints
- **Dispute:** Human can "flag" a missed alert, forcing the agent to re-evaluate its Noise Score.
- **Teaching:** Human can explicitly teach the agent "Always alert on X".
