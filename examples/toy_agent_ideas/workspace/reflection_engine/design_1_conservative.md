# Design 1: The Git-Hook Tutor

## Purpose
To prevent developers from committing code containing their own historically recurring errors. It acts as a personalized, local regression test suite for "human bugs".

## Loop Structure
1. **Trigger:** User runs `git commit`. The agent is invoked via a git `pre-commit` hook.
2. **Analysis:**
   - The agent reads the staged diff (`git diff --cached`).
   - It iterates through a local library of "Anti-Patterns" (regexes) that have been historically associated with bug fixes.
3. **Decision:**
   - If a match is found (e.g., using `var` instead of `let`, or a specific deprecated API call that caused issues before), it blocks the commit.
   - It displays a message: "You are making a mistake you've made 3 times before. See commit `abc1234`."
4. **Learning (Offline):**
   - A separate background process runs weekly.
   - It scans `git log` for commit messages containing "fix", "bug", "oops".
   - It analyzes the *diff* of those commits to find what was removed (the error) and what was added (the fix).
   - It generates new Regex patterns and adds them to the library.

## Tool Usage
- **shell:** To execute `git diff`, `git log`, and manage hooks.
- **grep:** To match patterns in the staged code.
- **filesystem:** To store the `anti_patterns.json` and read the codebase.

## Memory Architecture
- **Filesystem-based:** Simple JSON or YAML file (`.personal_linter_rules`) stored in the home directory or project root.
- **Why:** Low latency requirement for pre-commit hooks. No need for a complex graph.

## Failure Modes
- **False Positives:** The regex is too broad and blocks valid code.
  - *Recovery:* User can use `git commit --no-verify` to bypass. The agent logs this bypass and "weakens" the rule.
- **Performance:** If the history analysis takes too long, the hook becomes annoying.
  - *Mitigation:* The learning phase is asynchronous/offline. The hook only does fast regex matching.

## Human Touchpoints
- **Review:** The agent proposes new rules: "I noticed you fixed a NullPointer in `User.ts`. Should I add a rule to forbid `user.name` without `?`?"
- **Override:** Explicit bypass of the hook.
