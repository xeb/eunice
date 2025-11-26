# Design 3: The Career Strategist (Hybrid/Aggressive)

## Purpose
An agent that acts as a **Talent Agent**. It not only tracks what you *did* (using Design 2's mining) but compares it to what the *market wants* (via Web Search). It autonomously suggests: "You used React in 90% of projects, but the market is asking for Next.js 14. You have a gap here."

## Loop Structure
1. **Internal Audit:** Mines local code to build a "Current Skill Profile" (Memory Graph).
2. **Market Research:** Uses `web_search` to scrape job boards (LinkedIn, HN Hiring) for "Senior Engineer" roles.
3. **Gap Analysis:** Compares "Current Profile" vs "Market Demand".
4. **Resume Optimization:** Rewrites your Resume Markdown file to highlight skills that overlap with high-demand keywords, suppressing niche skills that are "out of fashion."
5. **Upskill Prompting:** Creates "Learning Tasks" in your todo list (e.g., "Refactor project X to use TypeScript to increase marketability").

## Tool Usage
- **grep/filesystem:** Skill mining.
- **memory:** Storing User Profile vs Market Profile.
- **web:** Searching for "React developer salary", "top skills 2025".
- **text-editor:** Rewriting the Resume/CV file.

## Memory Architecture
- **Dual Graph:**
  - **Internal:** What I know (Evidence-backed).
  - **External:** What the market values (Trend-backed).
- **Alignment Metric:** A score calculating "Market Fit."

## Failure Modes
- **Hallucination:** Might suggest skills the user *hates* just because they are popular.
- **Resume Bloat:** Keyword stuffing.
- **Recovery:** User locks specific resume sections from auto-editing.

## Human Touchpoints
- **Strategy Session:** User defines "Target Role" (e.g., "I want to be an ML Engineer, not Web Dev").
- **Approval:** Agent proposes resume changes; User accepts/rejects.

## Pros/Cons
- **Pros:** Directly impacts career growth/salary; proactive; closes the loop between coding and career.
- **Cons:** High complexity; requires trust to let AI edit resume; web scraping fragility.
