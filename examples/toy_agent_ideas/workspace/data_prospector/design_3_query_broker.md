# Design 3: The Query Broker (Virtual Data Lake)

## Purpose
To provide a unified SQL-like interface over the filesystem. Instead of just finding files, it *answers questions* by orchestrating on-the-fly data pipelines.

## Loop Structure
1. **Schema Mapping**: Maintain a lightweight map of Table Name -> File Path in Memory.
2. **Query Parsing**: User inputs a pseudo-SQL query: `SELECT * FROM users u JOIN orders o ON u.id = o.uid`.
3. **Plan Generation**:
   - Resolve `users` to `~/data/users.csv` and `orders` to `~/downloads/orders.json`.
   - Determine necessary tools (csvkit, jq, sqlite3).
4. **Execution**:
   - Construct a shell pipeline: `join -t, -1 1 -2 2 <(sort users.csv) <(jq_parse orders.json | sort)`.
   - Or: Import both into an ephemeral SQLite DB and run the query.
5. **Presentation**: Return the result set to the user.

## Tool Usage
- **shell**: Heavy use of `sqlite3` (import mode), `jq`, `csvkit`, `duckdb` (if available, else standard unix tools).
- **memory**: "Symbol Table" for aliases (e.g., "revenue_sheet" -> "2024_rev_final_v2.xlsx").
- **filesystem**: Read streams for piping.

## Memory Architecture
- **Nodes**:
  - `Alias`: {name, underlying_file_path}
  - `QueryLog`: {query_string, timestamp, result_count}

## Failure Modes
- **Missing Tools**: `duckdb` or `csvkit` not installed. *Fallback*: Use standard `awk`/`join` (harder to handle edge cases).
- **Data Dirtiness**: CSVs with bad quoting break pipelines. *Recovery*: Try to sanitize with `sed` before processing.

## Human Touchpoints
- **Interactive Querying**: The primary mode is conversational data retrieval.
- **Alias Definition**: "Hey, call this file 'Q3 Report'".
