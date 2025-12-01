# Tool Filtering Example

This example demonstrates fine-grained tool access control using pattern matching with wildcards.

## Overview

Three agents with different tool access:

| Agent | Role | Tools |
|-------|------|-------|
| `root` | Coordinator | None (delegates only) |
| `reader` | Read-only access | `fs_read_file`, `fs_list_directory`, etc. |
| `writer` | Write-only access | `fs_write_file`, `fs_edit_file`, etc. |

## Pattern Syntax

Tool patterns support `*` wildcards:

```toml
tools = ["fs_read_file"]           # Exact match
tools = ["fs_*"]                   # All fs_ tools
tools = ["*_file"]                 # All tools ending in _file
tools = ["fs_*_file"]              # fs_ prefix AND _file suffix
```

## Usage

```bash
# See all tools and which agents can access them
eunice --list-tools

# Example output:
# Discovered tools (12):
#
#   fs_create_directory [agents: writer]
#   fs_directory_tree [agents: reader]
#   fs_edit_file [agents: writer]
#   fs_get_file_info [agents: reader]
#   fs_list_directory [agents: reader]
#   fs_move_file [agents: writer]
#   fs_read_file [agents: reader]
#   fs_read_multiple_files [agents: reader]
#   fs_search_files [agents: reader]
#   fs_write_file [agents: writer]
#   ...

# Read a file (root delegates to reader)
eunice "Read the contents of notes.txt"

# Write a file (root delegates to writer)
eunice "Create a file called output.txt with 'Hello World'"

# Interactive mode
eunice -i
```

## How It Works

1. The `root` agent has no direct tool access (`tools = []`)
2. It can only invoke `reader` and `writer` agents
3. `reader` has specific read-related tools listed
4. `writer` has specific write-related tools listed
5. Pattern matching ensures each agent only sees their allowed tools

## Key Concept

This separation of concerns is useful for:
- **Security**: Limit what each agent can do
- **Clarity**: Each agent has a focused purpose
- **Auditability**: Clear boundaries on capabilities
