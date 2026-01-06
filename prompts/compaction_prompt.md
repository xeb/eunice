## Context Compression Task

Your task is to create a detailed summary of the conversation so far. This summary will be used as context when continuing the conversation, so preserve ALL critical information.

### What to Preserve
- **User's original request**: The task they asked for
- **What was accomplished**: Files created/modified, commands run, decisions made
- **Current work in progress**: Any incomplete tasks
- **Files involved**: List of files read, written, or modified with brief descriptions
- **Tool results summary**: Key outputs from tool calls (errors, important data)
- **Next steps**: What remains to be done
- **Key constraints**: Any user preferences, requirements, or limitations mentioned
- **Unresolved issues**: Errors, bugs, or blockers encountered

### What to Discard
- Redundant tool outputs (keep summaries only)
- Verbose file contents (keep filenames and key snippets)
- Repetitive conversation turns
- Debug output no longer relevant

### Format
Produce a structured summary that another instance of this model can use to seamlessly continue the conversation:

```
## Conversation Summary (Compressed Context)

**Original Task**: [one-line description]

**Completed**:
- [item 1]
- [item 2]

**In Progress**:
- [current task]

**Files**:
- `path/to/file.rs`: [brief description of changes]

**Key Decisions**:
- [decision 1 and rationale]

**Next Steps**:
1. [step 1]
2. [step 2]

**Notes**:
- [any important context]
```

Now summarize the following conversation:
