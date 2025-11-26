# Design 1: The Cataloger (Metadata Indexer)

## Purpose
To create a searchable metadata index of all structured data files on the local machine, solving the "Dark Data" problem where users forget where specific datasets are stored or what they contain.

## Loop Structure
1. **Discovery**: Walk the filesystem (within allowed paths) to find structured files (`.csv`, `.json`, `.sqlite`, `.parquet`, `.xlsx`).
2. **Extraction**: For each file:
   - Identify format.
   - Extract schema (Headers for CSV, Keys for JSON, Tables for SQLite).
   - Generate a "Schema Signature" (list of fields).
3. **Indexing**: Store the file path and its Schema Signature in the Memory Graph.
   - Entity: `File` (Path, Size, Modified)
   - Entity: `Schema` (Hash of columns)
   - Relation: `File` --(hasSchema)--> `Schema`
   - Relation: `Schema` --(containsField)--> `Field`
4. **Maintenance**: Watch for file changes (via polling or `stat` checks) and update the graph.

## Tool Usage
- **filesystem**: `list_directory` (recursive), `read_text_file` (head only for CSV headers).
- **shell**: Use `jq` for JSON keys, `sqlite3` for schema dump, `file` for type detection.
- **memory**: Store the file-to-schema mappings.
- **grep**: Not used in this variant (relying on structured parsers).

## Memory Architecture
- **Nodes**:
  - `File`: {path, type, last_scanned}
  - `Field`: {name, inferred_type}
- **Edges**:
  - `File` -> `HAS_FIELD` -> `Field`
- **Search**: User queries "Where is 'email'?" -> Agent queries Memory for `Field(name='email')` -> Returns connected `File` nodes.

## Failure Modes
- **Parsing Errors**: Malformed JSON/CSV. *Recovery*: Tag file as `unparseable` in Memory and skip.
- **Permission Denied**: *Recovery*: Log and ignore.
- **Large Files**: *Recovery*: Only read the first 1KB/header line.

## Human Touchpoints
- **Configuration**: User sets `allowed_directories`.
- **Query**: User asks "Find data about X".
