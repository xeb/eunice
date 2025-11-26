# Design 3: The Runtime Invariant Observer (Hybrid)

## Purpose
Instead of static analysis or active testing, this agent "watches" the system run (e.g., via logs or attached debugger) to *discover* what is actually true, then updates the documentation to match reality.

## Loop Structure
1. **Instrumentation**:
   - Injects "Observer Hooks" into the code (logging probes) using `text-editor`.
   - Or parses existing logs/traces.
2. **Observation**:
   - Accumulates data (e.g., "Function X returned values [1, 5, 100]").
   - Infers invariants (e.g., "Return value is always positive").
3. **Cross-Reference**:
   - Checks if these inferred invariants match the documentation.
   - **Drift Detection**: If code *always* returns positive, but docs say "returns integer", it suggests narrowing the type in docs.
   - **Violation Detection**: If logs show a `null` but docs say "non-nullable", it flags a bug.
4. **Annotation**:
   - Adds JSDoc/Docstring updates to the code via `text-editor`.

## Tool Usage
- `text-editor`: Injecting logging/probes.
- `shell`: Running the application or test suite to generate traffic.
- `memory`: Storing statistical distributions of observed values.

## Memory Architecture
- **Entities**: `Function`, `Variable`.
- **Observations**: `Min`, `Max`, `IsNullable`, `UniqueCount`.
- **Inference**: "Confidence: 99% that `user_id` is monotonic increasing."

## Failure Modes
- **Observer Effect**: Instrumentation slows down the app.
- **Insufficient Coverage**: Running only happy-path tests leads to "Overfitting" invariants (e.g., "Input is always 5" just because we only tested 5).
- **Code Dirt**: Leaving instrumentation code behind.

## Human Touchpoints
- **Permission**: Explicit approval before injecting probes.
- **Cleanup**: Command to strip all probes.
