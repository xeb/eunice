# Design 3: The Code Rosetta (Hybrid)

## Purpose
To modernize the *implementation* while preserving the *interface*. It effectively "hollows out" legacy code, turning it into a thin wrapper around modern standard libraries. This prevents "Not Invented Here" while avoiding "Breaking Change" trauma.

## Loop Structure
1. **Mapping**: Analyze a local utility function's inputs and outputs.
2. **Matching**: Find a standard library function with a *similar* but not identical signature.
3. **Bridge Generation**: Write an "Adapter" body.
    - *Old*: `def my_sort(list): ... custom bubble sort ...`
    - *New*: `def my_sort(list): return sorted(list, key=...)`
4. **Verification**: Run specific unit tests for that function.
5. **Evolution**:
    - Mark the function as `@deprecated(use="sorted")`.
    - Update call sites lazily over time.

## Tool Usage
- **grep**: Find call sites to assess impact.
- **text-editor**: Rewrite function bodies to be wrappers.
- **memory**: Track the "Hollowing Out" percentage of the codebase.

## Memory Architecture
- **Entities**: `WrapperFunction`, `UnderlyingTech`.
- **Relations**: `wraps`, `modernizes`.
- **Insight**: "We have 5 different wrappers for `axios`, maybe we should just use `axios` directly."

## Failure Modes
- **Performance Overhead**: The wrapper adds unnecessary casting/conversion.
- **Leaky Abstractions**: The new library throws different exceptions than the old code.
- **Recovery**: The agent wraps the new call in a `try/catch` block that mimics the old exception behavior, if documented.

## Human Touchpoints
- **Silent Upgrade**: Can run in background for minor internal utils.
- **Deprecation Notice**: Notifies humans when a wrapper is ready to be removed entirely.
