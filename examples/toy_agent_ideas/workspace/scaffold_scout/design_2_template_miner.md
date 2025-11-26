# Design 2: The Template Miner (Innovative)

## Purpose
To proactively reduce boilerplate by converting repetitive code into reusable templates (e.g., Hygen, Plop, or shell scripts) automatically.

## Loop Structure
1. **Change Detection:** Monitors the file system for new directories with multiple files.
2. **Clone Detection:**
   - When a new structure appears (e.g., `src/features/search/`), it compares it against existing structures (e.g., `src/features/auth/`).
3. **Abstraction Engine:**
   - If a match is found, it performs a "Diff-to-Variable" analysis.
   - Identifies tokens that differ (e.g., "Search" vs "Auth") and tokenizes them as `{{FeatureName}}`.
   - Identifies tokens that are derived (e.g., "SEARCH_REQUEST" vs "AUTH_REQUEST") and marks them as transformations.
4. **Template Generation:**
   - Writes a new generator definition to `.scaffold/templates/auto-generated-feature`.
   - Notifies the user: "I noticed you created a Feature. I made a template for next time."

## Tool Usage
- **shell:** `find`, `sed`, `awk` for token replacement.
- **memory:** Tracks "Trusted Templates" vs "Candidate Templates".
- **text-editor:** Writes the template files.

## Memory Architecture
- **Nodes:** `TemplateDefinition`, `Variable` (Name, Type).
- **Observations:** "User accepted template X", "User modified generated file Y" (Learning from drift).

## Failure Modes
- **Over-Abstraction:** Replaces common words that happened to differ but shouldn't be variables.
- **Syntax Errors:** Generated templates might produce invalid code if the abstraction is naive.
- **Recovery:** User simply deletes the bad template.

## Human Touchpoints
- **Notification:** "New template created."
- **Usage:** User explicitly runs the template generator command.
