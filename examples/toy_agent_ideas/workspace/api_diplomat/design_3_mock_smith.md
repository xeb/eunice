# Design 3: The Shadow Mock (Simulation First)

## Purpose
To maintain a "Digital Twin" of external dependencies. Instead of fixing the client, this agent focuses on keeping a local Mock Server (e.g., WireMock) perfectly synchronized with production reality, ensuring developers always test against accurate simulations.

## Problem Domain
Development often breaks because local mocks are outdated compared to the real production API. "It worked on my machine" happens because "my machine" was mocking the API as it existed 6 months ago.

## Core Tools
- **fetch**: To capture "Golden Master" responses from production (or a sandbox environment).
- **filesystem**: To write JSON/YAML files for the Mock Server configuration.
- **memory**: To track the "Freshness" of each mocked endpoint.
- **shell**: To restart the mock server container.

## Loop Structure
1.  **Census**: Scans the codebase to find all external API calls.
2.  **Sampling**: Periodically (or on demand) sends requests to the *real* API to record the shape of the response.
3.  **Diffing**: Compares the real response with the currently stored Mock definition.
4.  **Synchronization**:
    - If the real API response differs (e.g., new field added), it updates the Mock definition files.
    - If the real API is down, it does nothing (preserving the last known good state).
5.  **Reporting**: Notifies developers: "Mock updated: User object now includes 'avatar_url'."

## Memory Architecture
- **State**: Stores the hash of the response body for every endpoint.
- **Frequency**: Learns which endpoints change often (Volatility Index) and polls them more frequently.

## Failure Modes
- **PII Leakage**: Accidentally recording real user data into a shared mock file. *Mitigation*: Automated sanitization/masking of PII fields (email, phone, ids) before writing to filesystem.
- **Stateful Conflicts**: Replaying a recorded sequence that relies on server-side state (e.g., an ID that no longer exists). *Mitigation*: Focus on structural/schema mocking rather than data-value mocking.

## Human Touchpoints
- **Sanitization Rules**: Humans define what data patterns (Regex) must be scrubbed.
- **Conflict Resolution**: If the Mock update breaks local tests, the human must decide if the tests are wrong or if the API change is a breaking bug.
