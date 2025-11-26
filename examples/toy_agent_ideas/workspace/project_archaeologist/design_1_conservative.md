# Design 1: The Code Librarian (Conservative)

## Purpose
A read-only, on-demand documentation generator that creates and maintains a high-quality static documentation site (Markdown/Mermaid) for legacy codebases. It serves as a "first pass" tool for developers entering a new project.

## Loop Structure
**Trigger:** Manual invocation (CLI) or CI/CD pipeline hook (e.g., nightly build).
1. **Scan:** Iterates through the file system to list target files.
2. **Parse:** Reads core files (prioritizing by size/activity).
3. **Analyze:** Uses LLM and regex to extract:
   - High-level module descriptions
   - Dependency graphs (imports)
   - TODO/FIXME comments
   - Public API signatures
4. **Generate:** Writes structured Markdown files to a `docs/` directory.
   - `docs/modules/`
   - `docs/dependencies.mmd` (Mermaid chart)
   - `docs/technical_debt.md`
5. **Report:** Outputs a summary of changes since the last run.

## Tool Usage
- **filesystem:** Read code, write documentation.
- **grep:** Fast searching for keywords (TODO, FIXME, import, class).
- **shell:** Execute git commands to find recently changed files.

## Memory Architecture
- **Stateless/Filesystem:** Relies entirely on the current state of the codebase and previous documentation files. No persistent graph database. This ensures portability (commit the docs to Git).

## Failure Modes
- **Outdated Docs:** If not run frequently, docs drift from reality.
- **Parsing Errors:** Regex/LLM parsing might miss complex language constructs or hallucinate.
- **Recovery:** Simply re-run the tool. It overwrites old docs.

## Human Touchpoints
- **Review:** Humans read the generated docs.
- **Configuration:** Users invoke the tool and configure exclusion patterns via config file.
