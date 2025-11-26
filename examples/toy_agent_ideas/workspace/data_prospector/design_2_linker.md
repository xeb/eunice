# Design 2: The Linker (Topology Discoverer)

## Purpose
To go beyond simple indexing by discovering *relationships* between disparate datasets. It identifies implicit joins (e.g., "The 'id' in users.csv matches 'user_id' in orders.json") to build a "Data Topology" of the filesystem.

## Loop Structure
1. **Indexing**: Same as Design 1 (Find & Extract Schemas).
2. **Profiling**: For each field in each file:
   - Sample the data (e.g., first 100 rows).
   - Generate a "Value Signature" (Min/Max, Regex pattern, Cardinality, Bloom Filter or Hash of unique values).
3. **Linkage Analysis**:
   - Compare "Value Signatures" across the Knowledge Graph.
   - If Field A (File 1) and Field B (File 2) share >80% value overlap and similar data types:
     - Create Relation: `File 1` --(canJoinOn {field: A, otherField: B})--> `File 2`.
4. **Insight Generation**:
   - Report "Orphaned Data" (files that don't link to anything).
   - Report "Hub Data" (files that link to many others, likely core referential tables).

## Tool Usage
- **shell**: `awk`, `sort`, `uniq` for generating frequency distributions and signatures. `comm` for finding overlaps.
- **memory**: Store `ValueSignature` on Field nodes and `POSSIBLE_JOIN` relations between Files.
- **grep**: Used to validate if a specific ID exists in another file without loading the whole file.

## Memory Architecture
- **Nodes**:
  - `Field`: {name, type, sample_hash, cardinality}
- **Relations**:
  - `Field` --(matchesDistribution)--> `Field` (with score property)

## Failure Modes
- **False Positives**: Integer IDs (1, 2, 3) matching everywhere. *Mitigation*: Require high cardinality or specific column name similarity + value overlap.
- **Performance**: Profiling every file is slow. *Recovery*: Lazy profiling (only profile when machine is idle or on demand).

## Human Touchpoints
- **Verification**: User confirms "Yes, these files are related".
- **Exploration**: User asks "What files relate to Project X?" and the agent traverses the join graph.
