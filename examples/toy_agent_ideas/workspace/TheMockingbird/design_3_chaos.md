# Design 3: The Jester (Chaos Proxy)

## Purpose
A "Resilience Trainer" for your application. It proxies traffic to the *real* API but autonomously injects realistic failures based on "Chaos Policies". It learns from `web` research what common failures look like for that specific service (e.g., "AWS S3 503 error format").

## Loop Structure
1. **Setup:** User configures target API URL.
2. **Research:** Agent searches `web` for "Common [Service] error responses" and "Rate limit headers for [Service]".
3. **Plan:** Stores "Chaos Profiles" in **Memory** (e.g., "Latency Spike", "JSON Corruption", "Auth Failure").
4. **Proxy:** Runs a proxy server.
   - 90% of requests pass through untouched.
   - 10% are intercepted.
5. **Intervention:** When intercepted:
   - Agent selects a Chaos Profile from Memory.
   - Modifies the response (adds 5000ms delay, changes status to 502, truncates JSON).
6. **Report:** Logs the "Attack" to a Markdown file so the user knows *why* their app crashed.

## Tool Usage
- **web:** Find specific error schemas (XML vs JSON) for the target service.
- **memory:** Store the library of "Bad Behaviors".
- **shell:** Run the proxy (e.g., `mitmproxy` with a script).

## Memory Architecture
- **Entities:** `ChaosScenario` (name: "Database Timeout").
- **Observations:** Template for the error response.

## Failure Modes
- **Over-Aggression:** Breaks the app so bad development stops.
- **Security:** Accidentally logs real API keys from the proxied traffic.
- **Recovery:** "Safe Mode" toggle to disable chaos.

## Human Touchpoints
- **Configuration:** Setting the "Failure Rate" (0% to 100%).
- **Post-Mortem:** Reading the report of injected errors.
