# Git Helper Skill

## Description
Assist with Git operations including commits, branches, merges, rebases, and repository management. Follow best practices for version control.

## Instructions

### Checking Repository Status
```bash
git status
git log --oneline -10
git branch -a
```

### Making Commits
1. Stage specific files (avoid git add . to prevent accidental commits)
2. Write clear commit messages following conventional commits
3. Use the format: <type>: <description>

Types: feat, fix, docs, style, refactor, test, chore

### Branch Operations
```bash
git checkout -b feature/name    # Create new branch
git switch main                 # Switch to main
git merge feature/name          # Merge branch
git branch -d feature/name      # Delete merged branch
```

### Handling Conflicts
1. Identify conflicting files with git status
2. Open files and look for conflict markers (<<<<, ====, >>>>)
3. Edit to resolve, then git add and continue

### Best Practices
- Commit early and often
- Write meaningful commit messages
- Keep commits atomic (one logical change per commit)
- Never force push to shared branches
- Use branches for features and fixes

## Example Usage

User: "Help me commit my changes"

1. Run git status to see changes
2. Run git diff to review what changed
3. Stage appropriate files
4. Create a descriptive commit message
5. Commit the changes
