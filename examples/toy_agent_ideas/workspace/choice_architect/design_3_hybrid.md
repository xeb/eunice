# Design 3: The Ambient Mirror (Hybrid)

## Purpose
To influence behavior through peripheral perception / subliminal cues. Instead of blocking or changing configs, it alters the *environment's appearance* to reflect the system's state, leveraging the "Broken Windows Theory".

## Loop Structure
1. **Sensing**: Analyze codebase health metrics (coverage, complexity, TODO count, open issues).
2. **Mapping**: Map metrics to "Environmental Variables".
   - **Health** -> **Prompt Color** (Green = Good, Red = Decay).
   - **Tech Debt** -> **ASCII Art** in the terminal header (Clean vs. Glitchy).
   - **Staleness** -> **File Ordering** (Sort `ls` output to show stale files first? Or dim them?).
3. **Rendering**:
   - Update `.bashrc` or shell prompt exports.
   - Update IDE theme colors (if accessible via API/Config).
   - Generate a "Daily Briefing" file that sits on the Desktop/Root.

## Tool Usage
- **grep**: Count "TODO"s, check complexity.
- **shell**: `git log` analysis.
- **filesystem**: Modify shell profiles, generate "Mirror" artifacts.

## Memory Architecture
- **Time Series**: Track metrics over time to show *trends* (Getting better vs. Getting worse).
- **Persistence**: Store the "Current Mood" of the codebase.

## Failure Modes
- **Distraction**: Visual changes become annoying.
  - *Recovery*: Subtlety settings (High/Medium/Low contrast).
- **Desensitization**: User ignores the red prompt after a week.
  - *Recovery*: Novelty. Change the signal channel (e.g., from color to emoji, or text to sound).

## Human Touchpoints
- Minimal. The agent is a "Dashboard" that is woven into the tools themselves.
