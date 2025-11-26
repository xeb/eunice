# Design 1: The Aesthetic Collector (Conservative)

## Purpose
A straightforward background agent that turns a text-based "wishlist" of visual concepts into a local folder of reference images. It automates the tedious "Search -> Right Click Save" loop for designers.

## Loop Structure
1. **Watch**: Monitor `inspiration_wishlist.txt` for new lines (e.g., "glowing mushroom forest", "1980s computer lab").
2. **Search**: When a new line appears, use `web_brave_image_search` to find 10-20 high-quality images.
3. **Acquire**: Use `fetch` to download the images to `workspace/aesthetic_curator/downloads/<concept_name>/`.
4. **Log**: Write a `metadata.json` in the folder with source URLs and alt text.
5. **Notify**: Update a `status.md` file indicating new items are ready for review.

## Tool Usage
- **filesystem**: Reading the wishlist, writing image files, creating directories.
- **web**: `web_brave_image_search` for finding content.
- **fetch**: Downloading the actual image bytes.

## Memory Architecture
- **Stateless**: Relies entirely on the filesystem state. It checks if a folder exists to avoid re-searching.

## Failure Modes
- **Broken Links**: `fetch` might fail if the image URL is protected or dead.
  - *Recovery*: Try the next result in the search list.
- **Disk Space**: Downloading too many images.
  - *Recovery*: Stop if quota exceeded (checked via `filesystem_list_directory_with_sizes`).

## Human Touchpoints
- **Input**: User edits `inspiration_wishlist.txt`.
- **Curation**: User manually deletes images they don't like from the folders.
