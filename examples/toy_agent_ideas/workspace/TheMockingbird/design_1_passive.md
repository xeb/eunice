# Design 1: The Echo (Passive Recorder)

## Purpose
A "zero-configuration" mock server that learns by watching. It sits in the background, ingesting HAR (HTTP Archive) files or proxy logs from your development session, and builds a deterministic lookup table for API responses.

## Loop Structure
1. **Monitor:** Watches a specific directory for new `.har` files or log dumps.
2. **Ingest:** Parses the logs to extract Request (Method, URL, Body) -> Response (Status, Body, Headers) pairs.
3. **Index:** Stores these pairs in the **Memory Graph**, treating the URL path as a Node and the specific Request/Response pair as an Observation.
4. **Serve:** Runs a lightweight local server (via `shell`). When it receives a request:
   - Hashes the request.
   - Lookups exact matches in Memory.
   - If found: Returns the stored response.
   - If not found: Returns 404 and logs a "Missing Path" alert.

## Tool Usage
- **filesystem:** Watch `logs/` folder.
- **memory:** Store `(RequestHash) -> (ResponsePayload)` mappings.
- **shell:** Run a simple Python `http.server` or Node `express` app that queries the Memory tool for responses.

## Memory Architecture
- **Entities:** `Endpoint` (name: "/api/v1/users"), `Interaction` (name: hash of request).
- **Relations:** `Endpoint` HAS_INTERACTION `Interaction`.
- **Observations:** The actual JSON body of the response.

## Failure Modes
- **Stale Data:** The real API changes, but the mock returns old data.
- **Dynamic Fields:** Timestamps or IDs in requests break the exact hash matching.
- **Recovery:** User must "flush" the memory or provide a new HAR file.

## Human Touchpoints
- **Training:** User must browse the real site once to generate traffic.
- **Playback:** User points their localhost app to the Mockingbird port.
