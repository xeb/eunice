# Design 1: The Chaote (Entropy Generator)

## Purpose
To destroy advertising profiles by injecting pure, high-volume random noise into the user's browsing history.

## Loop Structure
1. **Wake**: Runs every 15 minutes.
2. **Seed**: Fetches "Trending Searches" from Google/Bing or picks random words from a local dictionary.
3. **Execute**: Performs 10-50 rapid searches using `web_brave_web_search`.
4. **Sleep**: Waits for the next interval.

## Tool Usage
- **web_brave_web_search**: The primary weapon. Used to generate traffic.
- **filesystem_read_text_file**: Reads a `dictionary.txt` or `trends.json`.
- **shell_execute_command**: (Optional) To clear local cookies/cache after every run (scorched earth).

## Memory Architecture
- **Stateless**: The Chaote does not remember what it did. It is pure entropy.
- **Log-only**: Uses filesystem to log "I searched for 'Underwater Basket Weaving' at 14:00".

## Failure Modes
- **Bot Detection**: Ad networks easily identify non-human patterns (too fast, no mouse movement).
- **Rate Limiting**: Brave Search API or target sites block the IP.
- **Recovery**: If blocked, it sleeps for 6 hours.

## Human Touchpoints
- **None**: It is a "fire and forget" background daemon.

## Critique
Too primitive. Modern tracking algorithms can filter out "random noise" easily because it lacks semantic coherence (co-visitation patterns).
