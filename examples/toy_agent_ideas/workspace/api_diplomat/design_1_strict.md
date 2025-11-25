# Design 1: The Strict Constructionist (Schema Enforcement)

## Purpose
To act as a rigid "Border Control" agent that guarantees external APIs strictly adhere to their agreed-upon contracts (OpenAPI/Swagger specs). It prevents "drift" by immediately detecting unauthorized changes in response structures, headers, or status codes.

## Problem Domain
API integrations often fail silently when a third-party provider changes a field type, deprecates a parameter, or alters error responses without notice. This leads to runtime exceptions that are hard to debug.

## Core Tools
- **fetch**: To poll API endpoints and capture live responses.
- **filesystem**: To read the authoritative `openapi.yaml` or `swagger.json` contracts.
- **shell**: To run validation tools (like `spectral` or custom validators).
- **memory**: To log incidents and track "reliability scores" of external providers.

## Loop Structure
1.  **Load Contracts**: Reads all `*.spec.yaml` files in the `contracts/` directory.
2.  **Scheduled Patrol**: Every X hours, it executes a set of "safe" (GET/HEAD) requests defined in the spec.
3.  **Validation**: Compares the live response (headers, body, status) against the schema definition.
4.  **Verdict**:
    - *Match*: Updates the "Last Verified" timestamp in Memory.
    - *Mismatch*: Generates a "Drift Report" (Markdown) and optionally triggers a shell alert (e.g., Slack hook).

## Memory Architecture
- **Entities**: `Provider`, `Endpoint`, `SchemaField`.
- **Relations**: `Provider HAS Endpoint`, `Endpoint EXHIBITS Drift`.
- **Observations**: "Endpoint /users/1 returned string for 'id' instead of integer at 14:00."

## Failure Modes
- **False Positives**: API returns 500 due to downtime, interpreted as schema drift. *Recovery*: Retry logic and differentiation between network errors and schema errors.
- **Destructive Testing**: Accidental execution of non-idempotent methods (POST/DELETE). *Mitigation*: Strictly limited to GET/HEAD or specific test accounts.

## Human Touchpoints
- **Initial Setup**: Human must provide the `openapi.yaml` and auth credentials.
- **Drift Review**: Human must review the Drift Report and decide whether to update the spec (accept change) or contact the vendor (reject change).
