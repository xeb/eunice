# Agent: The Data Prospector

## Purpose
A "Local-First Data Catalog" that autonomously crawls the filesystem to discover, index, and link disparate structured data files (CSV, JSON, SQLite). It turns "Dark Data" (forgotten files) into a queryable Knowledge Graph, enabling users to understand their personal data topology without manual organization.

## Core Architecture
The design synthesizes **The Linker (Design 2)** with the **Query Broker (Design 3)** capabilities. It primarily builds a **Schema & Topology Graph** (Memory) to map what data exists and how it connects, but includes an **Execution Layer** (Shell) to prove those connections or answer specific questions.

### 1. The Discovery Loop (Background Daemon)
- **Crawl**: Recursively list allowed directories.
- **Fingerprint**: For every structured file:
  - **Metadata**: Path, Size, ModTime.
  - **Schema**: Column names/Keys.
  - **Profile**: Bloom filter of values for "Join Key" candidates (columns like 'id', 'email', 'uuid').
- **Graphing**:
  - Update Memory Graph with `File` and `Field` nodes.
  - Run **"Linkage Analysis"**: Check if File A's 'user_id' bloom filter overlaps with File B's 'id'.
  - Create `POSSIBLE_JOIN` edges with confidence scores.

### 2. The Interaction Layer (On-Demand)
- **Natural Language Search**: "Find all files with customer emails." -> Queries Graph for `Field(name ~= 'email')`.
- **Data Virtualization**: "Show me orders for customer X."
  - Agent uses the `POSSIBLE_JOIN` links to find the relevant files.
  - Generates a targeted `grep` or `sqlite3` command to fetch the specific rows across files.

## Tool Utilization
| Tool | Purpose |
|------|---------|
| **filesystem** | Reading file headers, streaming content for profiling. |
| **memory** | The "Metastore": Stores Schema topology, Join candidates, and Aliases. |
| **shell** | The "Engine": Uses `sqlite3`, `jq`, `awk` for profiling and ad-hoc querying. |
| **grep** | Fast existence checks ("Does this specific ID exist in that file?") to verify joins. |

## Memory Graph Structure
- **Entities**:
  - `File`: {path, format, last_indexed}
  - `Schema`: {hash}
  - `Field`: {name, inferred_type, cardinality_estimate}
  - `ValueSignature`: {bloom_filter_b64} (Optional/Sparse)
- **Relations**:
  - `File` --(implements)--> `Schema`
  - `Schema` --(hasField)--> `Field`
  - `Field` --(likelyJoinsWith {confidence: 0.9})--> `Field`

## Recovery & Safety
- **Resource Limits**: The "Profile" step only samples the first 10MB or 10k rows to prevent hanging on massive dumps.
- **Privacy**: Allows a `.prospectorignore` file to exclude sensitive directories.
- **Read-Only**: The agent never modifies user data files, only reads them.

## Insight
Most "Personal Data" tools fail because they require manual import. The Data Prospector succeeds by being **in-situ**: it indexes data *where it lives*, using a Graph to infer the structure that the user failed to document. It effectively brings "Data Lakehouse" concepts to the `~/Documents` folder.
