# Design 3: The Semantic Collage

## Purpose
To provide a visual and textual "Mood Board" of related and orthogonal concepts. This is less about structural problem solving (like Design 2) and more about "Ambient Inspiration" for creative writing or UI design.

## Loop Structure
1. **Sample**: Pick a random file or paragraph from the user's active project.
2. **Expand**: Use `web_brave_news_search` and `web_brave_image_search` to find:
   - Recent news related to the keywords.
   - Abstract images related to the *feel* of the text (e.g., if text is "fast, dark, aggressive", search for "cyberpunk city rain").
3. **Collage**:
   - Create a Markdown file `workspace/inspiration_board.md` with embedded image links and news headlines.
   - *Format:* "Here is the 'Vibe' of your current work, contrasted with..."
4. **Drift**:
   - Intentionally drift the search terms. If the user writes about "Coffee", search for "Tea ceremonies", then "Rituals", then "Religious ecstasy".
   - Present this "Drift Path" to the user.

## Tool Usage
- **web**: Heavy use of Image and News search.
- **filesystem**: Writing the Markdown Collage.

## Memory Architecture
- **Session-based**: Keeps track of the "Drift Path" so it doesn't repeat or loop back immediately.

## Failure Modes
- **Distraction**: Images can be distracting.
- **Copyright/Broken Links**: Image URLs might expire or be hotlink-protected.

## Human Touchpoints
- **Visual**: The user simply keeps the `inspiration_board.md` open in a split pane (rendered preview) while working.
