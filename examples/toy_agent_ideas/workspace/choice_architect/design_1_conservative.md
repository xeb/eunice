# Design 1: The Config Curator (Conservative)

## Purpose
To subtly improve developer productivity and code quality by managing the "default settings" of the development environment. It ensures that the "easiest" path is always the compliant/productive one.

## Loop Structure
1. **Observation**: Periodically (e.g., hourly) scan the codebase structure, git log, and project configuration files (`.vscode`, `.editorconfig`, `package.json`).
2. **Analysis**: Compare current project state against a "productivity heuristic" database (stored in Memory).
   - *Example*: "This is a Typescript project, but 'strict' mode is off."
   - *Example*: "Users frequently edit files in `legacy/` but there are no linting rules there."
3. **Intervention**:
   - Update `.vscode/settings.json` (e.g., enable format-on-save if mostly formatting commits).
   - Update `.git/config` (e.g., set up a commit template that encourages conventional commits).
   - Create/Update `.editorconfig` to enforce consistency.
4. **Reporting**: Silent operation. Changes are committed to a `chore/config-tuning` branch or applied locally.

## Tool Usage
- **filesystem**: Read/Write config files.
- **grep**: Analyze commit patterns (e.g., `grep "fix:"` to see if bug fixes are frequent, prompting stricter linting).
- **shell**: Run `git config` commands.

## Memory Architecture
- **Entities**: `Project`, `UserHabit`, `ConfigurationSetting`.
- **Relations**: `UserHabit` -> `REQUIRES` -> `ConfigurationSetting`.
- **Logic**: If `UserHabit` = "Forgets to format", THEN `ConfigurationSetting` = "formatOnSave: true".

## Failure Modes
- **Overwriting Preferences**: User manually sets a preference, Agent overwrites it.
  - *Recovery*: Agent checks for `# choice-architect: ignore` comments in config files.
- **Tool Incompatibility**: Enabling a setting that breaks the build.
  - *Recovery*: Agent runs a "verification command" (e.g., `npm run build`) after config change. If fail, revert.

## Human Touchpoints
- None required. Operates on the "Default Option" principle. User can always change it back, but the default is set optimally.
