use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::common::{error_content, text_content, McpServer, McpTool};

/// Configuration for the filesystem server
pub struct FilesystemServerConfig {
    pub allowed_directories: Vec<PathBuf>,
    pub verbose: bool,
}

impl FilesystemServerConfig {
    pub fn new(allowed_directories: Vec<PathBuf>, verbose: bool) -> Result<Self> {
        // Validate and resolve all directories
        let mut resolved_dirs = Vec::new();
        for dir in allowed_directories {
            let expanded = expand_home(&dir);
            let absolute = if expanded.is_absolute() {
                expanded
            } else {
                std::env::current_dir()?.join(&expanded)
            };

            // Try to resolve symlinks, fall back to absolute path
            let resolved = match fs::canonicalize(&absolute) {
                Ok(p) => p,
                Err(_) => absolute,
            };

            // Verify directory exists and is accessible
            let metadata = fs::metadata(&resolved)
                .with_context(|| format!("Cannot access directory: {}", resolved.display()))?;

            if !metadata.is_dir() {
                return Err(anyhow!("{} is not a directory", resolved.display()));
            }

            resolved_dirs.push(resolved);
        }

        if resolved_dirs.is_empty() {
            return Err(anyhow!(
                "At least one allowed directory must be specified"
            ));
        }

        Ok(Self {
            allowed_directories: resolved_dirs,
            verbose,
        })
    }
}

/// Expand ~ to home directory
fn expand_home(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

/// Validate that a path is within allowed directories
fn validate_path(path: &str, allowed_dirs: &[PathBuf]) -> Result<PathBuf> {
    let expanded = expand_home(Path::new(path));
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()?.join(&expanded)
    };

    // Try to resolve symlinks to get the real path
    let resolved = match fs::canonicalize(&absolute) {
        Ok(p) => p,
        Err(e) => {
            // For new files, check parent directory
            if e.kind() == std::io::ErrorKind::NotFound {
                if let Some(parent) = absolute.parent() {
                    let parent_resolved = fs::canonicalize(parent)
                        .with_context(|| format!("Parent directory does not exist: {}", parent.display()))?;

                    // Check if parent is within allowed directories
                    if !is_within_allowed(&parent_resolved, allowed_dirs) {
                        return Err(anyhow!(
                            "Access denied - parent directory outside allowed directories: {}",
                            parent_resolved.display()
                        ));
                    }
                    return Ok(absolute);
                }
            }
            return Err(anyhow!("Cannot access path: {} - {}", absolute.display(), e));
        }
    };

    // Check if resolved path is within allowed directories
    if !is_within_allowed(&resolved, allowed_dirs) {
        return Err(anyhow!(
            "Access denied - path outside allowed directories: {}",
            resolved.display()
        ));
    }

    Ok(resolved)
}

/// Check if a path is within any of the allowed directories
fn is_within_allowed(path: &Path, allowed_dirs: &[PathBuf]) -> bool {
    allowed_dirs.iter().any(|allowed| path.starts_with(allowed))
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Format timestamp for display
fn format_time(time: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// File information structure
#[derive(Serialize)]
struct FileInfo {
    size: u64,
    size_formatted: String,
    created: String,
    modified: String,
    accessed: String,
    is_directory: bool,
    is_file: bool,
    is_symlink: bool,
    permissions: String,
}

/// Directory entry with size
#[derive(Serialize)]
struct DirectoryEntry {
    name: String,
    is_directory: bool,
    size: u64,
}

/// Tree entry for directory_tree
#[derive(Serialize, Deserialize)]
struct TreeEntry {
    name: String,
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<TreeEntry>>,
}

/// Edit operation for edit_file
#[derive(Deserialize)]
struct EditOperation {
    #[serde(rename = "oldText")]
    old_text: String,
    #[serde(rename = "newText")]
    new_text: String,
}

/// Filesystem MCP server
pub struct FilesystemServer {
    config: FilesystemServerConfig,
}

impl FilesystemServer {
    pub fn new(config: FilesystemServerConfig) -> Self {
        Self { config }
    }

    fn allowed_dirs(&self) -> &[PathBuf] {
        &self.config.allowed_directories
    }

    // Tool implementations

    fn read_file(
        &self,
        path: &str,
        head: Option<usize>,
        tail: Option<usize>,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;

        // Check for conflicting parameters
        let has_head_tail = head.is_some() || tail.is_some();
        let has_offset_limit = offset.is_some() || limit.is_some();

        if has_head_tail && has_offset_limit {
            return Err(anyhow!(
                "Cannot combine head/tail with offset/limit parameters"
            ));
        }

        if head.is_some() && tail.is_some() {
            return Err(anyhow!("Cannot specify both head and tail parameters"));
        }

        if let Some(n) = tail {
            return self.tail_file(&valid_path, n);
        }

        if let Some(n) = head {
            return self.head_file(&valid_path, n);
        }

        if has_offset_limit {
            return self.read_file_range(&valid_path, offset.unwrap_or(1), limit);
        }

        fs::read_to_string(&valid_path)
            .with_context(|| format!("Failed to read file: {}", valid_path.display()))
    }

    fn read_file_range(&self, path: &Path, offset: usize, limit: Option<usize>) -> Result<String> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // offset is 1-indexed (line 1 is the first line)
        let skip_lines = offset.saturating_sub(1);

        let lines: Vec<String> = if let Some(n) = limit {
            reader
                .lines()
                .skip(skip_lines)
                .take(n)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            reader
                .lines()
                .skip(skip_lines)
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(lines.join("\n"))
    }

    fn tail_file(&self, path: &Path, num_lines: usize) -> Result<String> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        if file_size == 0 {
            return Ok(String::new());
        }

        let mut reader = BufReader::new(file);
        let chunk_size: i64 = 1024;
        let mut lines: Vec<String> = Vec::new();
        let mut position = file_size as i64;
        let mut remainder = String::new();

        while position > 0 && lines.len() < num_lines {
            let read_size = std::cmp::min(chunk_size, position);
            position -= read_size;

            reader.seek(SeekFrom::Start(position as u64))?;
            let mut buffer = vec![0u8; read_size as usize];
            reader.read_exact(&mut buffer)?;

            let chunk_text = String::from_utf8_lossy(&buffer).to_string();
            let combined = format!("{}{}", chunk_text, remainder);
            let mut chunk_lines: Vec<&str> = combined.split('\n').collect();

            // Save incomplete first line for next iteration
            if position > 0 && !chunk_lines.is_empty() {
                remainder = chunk_lines.remove(0).to_string();
            } else {
                remainder.clear();
            }

            // Add lines in reverse order (we're reading backwards)
            for line in chunk_lines.into_iter().rev() {
                if lines.len() < num_lines {
                    lines.insert(0, line.to_string());
                }
            }
        }

        // Add any remaining text
        if !remainder.is_empty() && lines.len() < num_lines {
            lines.insert(0, remainder);
        }

        Ok(lines.into_iter().take(num_lines).collect::<Vec<_>>().join("\n"))
    }

    fn head_file(&self, path: &Path, num_lines: usize) -> Result<String> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .take(num_lines)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(lines.join("\n"))
    }

    fn read_multiple_files(
        &self,
        paths: &[String],
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<String> {
        let results: Vec<String> = paths
            .iter()
            .map(|path| {
                match self.read_file(path, None, None, offset, limit) {
                    Ok(content) => format!("{}:\n{}\n", path, content),
                    Err(e) => format!("{}: Error - {}", path, e),
                }
            })
            .collect();

        Ok(results.join("\n---\n"))
    }

    fn write_file(&self, path: &str, content: &str) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;

        // Write atomically to prevent race conditions
        let temp_path = format!("{}.{}.tmp", valid_path.display(), std::process::id());
        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temp file: {}", temp_path))?;

        // If target exists and is different from temp, rename
        if valid_path.exists() {
            fs::rename(&temp_path, &valid_path)
                .with_context(|| format!("Failed to rename temp file to: {}", valid_path.display()))?;
        } else {
            fs::rename(&temp_path, &valid_path)
                .with_context(|| format!("Failed to create file: {}", valid_path.display()))?;
        }

        Ok(format!("Successfully wrote to {}", path))
    }

    fn edit_file(&self, path: &str, edits: Vec<EditOperation>, dry_run: bool) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;
        let original_content = fs::read_to_string(&valid_path)?;

        // Normalize line endings
        let mut content = original_content.replace("\r\n", "\n");

        // Apply edits sequentially
        for edit in edits {
            let old_text = edit.old_text.replace("\r\n", "\n");
            let new_text = edit.new_text.replace("\r\n", "\n");

            if content.contains(&old_text) {
                content = content.replacen(&old_text, &new_text, 1);
            } else {
                // Try whitespace-flexible matching
                let old_lines: Vec<&str> = old_text.lines().collect();
                let content_lines: Vec<&str> = content.lines().collect();
                let mut found = false;

                'outer: for i in 0..=content_lines.len().saturating_sub(old_lines.len()) {
                    let matches = old_lines.iter().enumerate().all(|(j, old_line)| {
                        content_lines.get(i + j)
                            .map(|content_line| old_line.trim() == content_line.trim())
                            .unwrap_or(false)
                    });

                    if matches {
                        // Replace the matched lines
                        let mut new_lines: Vec<String> = content_lines[..i]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();

                        // Preserve original indentation
                        let original_indent = content_lines[i]
                            .chars()
                            .take_while(|c| c.is_whitespace())
                            .collect::<String>();

                        for (j, new_line) in new_text.lines().enumerate() {
                            if j == 0 {
                                new_lines.push(format!("{}{}", original_indent, new_line.trim_start()));
                            } else {
                                new_lines.push(new_line.to_string());
                            }
                        }

                        new_lines.extend(
                            content_lines[i + old_lines.len()..]
                                .iter()
                                .map(|s| s.to_string())
                        );

                        content = new_lines.join("\n");
                        found = true;
                        break 'outer;
                    }
                }

                if !found {
                    return Err(anyhow!("Could not find exact match for edit:\n{}", edit.old_text));
                }
            }
        }

        // Create unified diff
        let diff = create_unified_diff(&original_content, &content, path);

        if !dry_run {
            // Write atomically
            let temp_path = format!("{}.{}.tmp", valid_path.display(), std::process::id());
            fs::write(&temp_path, &content)?;
            fs::rename(&temp_path, &valid_path)?;
        }

        Ok(format!("```diff\n{}\n```\n", diff))
    }

    fn create_directory(&self, path: &str) -> Result<String> {
        // For create_directory, we need to validate the path or find the first existing parent
        let expanded = expand_home(Path::new(path));
        let absolute = if expanded.is_absolute() {
            expanded
        } else {
            std::env::current_dir()?.join(&expanded)
        };

        // Find the first existing parent and validate that
        let mut check_path = absolute.clone();
        while !check_path.exists() {
            check_path = match check_path.parent() {
                Some(p) => p.to_path_buf(),
                None => break,
            };
        }

        if check_path.exists() {
            let resolved = fs::canonicalize(&check_path)?;
            if !is_within_allowed(&resolved, self.allowed_dirs()) {
                return Err(anyhow!(
                    "Access denied - path outside allowed directories: {}",
                    absolute.display()
                ));
            }
        }

        fs::create_dir_all(&absolute)
            .with_context(|| format!("Failed to create directory: {}", absolute.display()))?;
        Ok(format!("Successfully created directory {}", path))
    }

    fn list_directory(&self, path: &str) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;
        let entries = fs::read_dir(&valid_path)
            .with_context(|| format!("Failed to read directory: {}", valid_path.display()))?;

        let mut result: Vec<String> = Vec::new();
        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let prefix = if file_type.is_dir() { "[DIR]" } else { "[FILE]" };
            result.push(format!("{} {}", prefix, entry.file_name().to_string_lossy()));
        }

        result.sort();
        Ok(result.join("\n"))
    }

    fn list_directory_with_sizes(&self, path: &str, sort_by: &str) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;
        let entries = fs::read_dir(&valid_path)?;

        let mut detailed_entries: Vec<DirectoryEntry> = Vec::new();
        let mut total_size: u64 = 0;
        let mut total_files = 0;
        let mut total_dirs = 0;

        for entry in entries {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let is_dir = metadata.is_dir();
            let size = if is_dir { 0 } else { metadata.len() };

            if is_dir {
                total_dirs += 1;
            } else {
                total_files += 1;
                total_size += size;
            }

            detailed_entries.push(DirectoryEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                is_directory: is_dir,
                size,
            });
        }

        // Sort entries
        match sort_by {
            "size" => detailed_entries.sort_by(|a, b| b.size.cmp(&a.size)),
            _ => detailed_entries.sort_by(|a, b| a.name.cmp(&b.name)),
        }

        // Format output
        let mut result: Vec<String> = detailed_entries
            .iter()
            .map(|e| {
                let prefix = if e.is_directory { "[DIR]" } else { "[FILE]" };
                let size_str = if e.is_directory {
                    String::new()
                } else {
                    format!("{:>10}", format_size(e.size))
                };
                format!("{} {:30} {}", prefix, e.name, size_str)
            })
            .collect();

        result.push(String::new());
        result.push(format!("Total: {} files, {} directories", total_files, total_dirs));
        result.push(format!("Combined size: {}", format_size(total_size)));

        Ok(result.join("\n"))
    }

    fn directory_tree(&self, path: &str, exclude_patterns: &[String]) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;
        let tree = self.build_tree(&valid_path, &valid_path, exclude_patterns)?;
        Ok(serde_json::to_string_pretty(&tree)?)
    }

    fn build_tree(&self, root: &Path, current: &Path, exclude_patterns: &[String]) -> Result<Vec<TreeEntry>> {
        let entries = fs::read_dir(current)?;
        let mut result: Vec<TreeEntry> = Vec::new();

        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();
            let relative_path = entry_path.strip_prefix(root).unwrap_or(&entry_path);
            let relative_str = relative_path.to_string_lossy();

            // Check exclusion patterns
            let should_exclude = exclude_patterns.iter().any(|pattern| {
                matches_glob(pattern, &relative_str)
            });

            if should_exclude {
                continue;
            }

            let file_type = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                let children = self.build_tree(root, &entry_path, exclude_patterns)?;
                result.push(TreeEntry {
                    name,
                    entry_type: "directory".to_string(),
                    children: Some(children),
                });
            } else {
                result.push(TreeEntry {
                    name,
                    entry_type: "file".to_string(),
                    children: None,
                });
            }
        }

        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    fn move_file(&self, source: &str, destination: &str) -> Result<String> {
        let valid_source = validate_path(source, self.allowed_dirs())?;
        let valid_dest = validate_path(destination, self.allowed_dirs())?;

        fs::rename(&valid_source, &valid_dest)
            .with_context(|| format!("Failed to move {} to {}", source, destination))?;

        Ok(format!("Successfully moved {} to {}", source, destination))
    }

    fn search_files(&self, path: &str, pattern: &str, exclude_patterns: &[String]) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;
        let mut results: Vec<String> = Vec::new();
        self.search_recursive(&valid_path, &valid_path, pattern, exclude_patterns, &mut results)?;

        if results.is_empty() {
            Ok("No matches found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }

    fn search_recursive(
        &self,
        root: &Path,
        current: &Path,
        pattern: &str,
        exclude_patterns: &[String],
        results: &mut Vec<String>,
    ) -> Result<()> {
        let entries = match fs::read_dir(current) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_path = entry.path();

            // Validate path is still within allowed directories
            if !is_within_allowed(&entry_path, self.allowed_dirs()) {
                continue;
            }

            let relative_path = entry_path.strip_prefix(root).unwrap_or(&entry_path);
            let relative_str = relative_path.to_string_lossy();

            // Check exclusion patterns
            let should_exclude = exclude_patterns.iter().any(|p| matches_glob(p, &relative_str));
            if should_exclude {
                continue;
            }

            // Check if matches search pattern
            if matches_glob(pattern, &relative_str) {
                results.push(entry_path.to_string_lossy().to_string());
            }

            // Recurse into directories
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                self.search_recursive(root, &entry_path, pattern, exclude_patterns, results)?;
            }
        }

        Ok(())
    }

    fn get_file_info(&self, path: &str) -> Result<String> {
        let valid_path = validate_path(path, self.allowed_dirs())?;
        let metadata = fs::metadata(&valid_path)?;
        let symlink_metadata = fs::symlink_metadata(&valid_path)?;

        let info = FileInfo {
            size: metadata.len(),
            size_formatted: format_size(metadata.len()),
            created: metadata.created().map(format_time).unwrap_or_else(|_| "Unknown".to_string()),
            modified: metadata.modified().map(format_time).unwrap_or_else(|_| "Unknown".to_string()),
            accessed: metadata.accessed().map(format_time).unwrap_or_else(|_| "Unknown".to_string()),
            is_directory: metadata.is_dir(),
            is_file: metadata.is_file(),
            is_symlink: symlink_metadata.file_type().is_symlink(),
            permissions: format!("{:o}", metadata.permissions().mode() & 0o777),
        };

        let result = format!(
            "size: {}\nsize_formatted: {}\ncreated: {}\nmodified: {}\naccessed: {}\nis_directory: {}\nis_file: {}\nis_symlink: {}\npermissions: {}",
            info.size, info.size_formatted, info.created, info.modified, info.accessed,
            info.is_directory, info.is_file, info.is_symlink, info.permissions
        );

        Ok(result)
    }

    fn list_allowed_directories(&self) -> String {
        let dirs: Vec<String> = self.allowed_dirs()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        format!("Allowed directories:\n{}", dirs.join("\n"))
    }
}

/// Simple glob matching (supports * and **)
fn matches_glob(pattern: &str, path: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    matches_glob_recursive(&pattern_parts, &path_parts)
}

fn matches_glob_recursive(pattern: &[&str], path: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    let p = pattern[0];

    if p == "**" {
        // ** matches zero or more path segments
        if matches_glob_recursive(&pattern[1..], path) {
            return true;
        }
        if !path.is_empty() && matches_glob_recursive(pattern, &path[1..]) {
            return true;
        }
        return false;
    }

    if path.is_empty() {
        return false;
    }

    if matches_segment(p, path[0]) {
        matches_glob_recursive(&pattern[1..], &path[1..])
    } else {
        false
    }
}

fn matches_segment(pattern: &str, segment: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let mut pattern_chars = pattern.chars().peekable();
    let mut segment_chars = segment.chars().peekable();

    while let Some(p) = pattern_chars.next() {
        match p {
            '*' => {
                // * matches any sequence of characters within a segment
                if pattern_chars.peek().is_none() {
                    return true;
                }
                // Try matching remaining pattern at each position
                let remaining_pattern: String = pattern_chars.collect();
                let mut remaining_segment: String = segment_chars.collect();
                while !remaining_segment.is_empty() {
                    if matches_segment(&remaining_pattern, &remaining_segment) {
                        return true;
                    }
                    remaining_segment = remaining_segment.chars().skip(1).collect();
                }
                return matches_segment(&remaining_pattern, "");
            }
            '?' => {
                if segment_chars.next().is_none() {
                    return false;
                }
            }
            c => {
                if segment_chars.next() != Some(c) {
                    return false;
                }
            }
        }
    }

    segment_chars.next().is_none()
}

/// Create a simple unified diff
fn create_unified_diff(original: &str, modified: &str, filename: &str) -> String {
    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();

    let mut diff = String::new();
    diff.push_str(&format!("--- {}\n", filename));
    diff.push_str(&format!("+++ {}\n", filename));

    // Simple line-by-line diff
    let max_len = std::cmp::max(original_lines.len(), modified_lines.len());
    let mut i = 0;
    while i < max_len {
        let orig = original_lines.get(i).copied();
        let modi = modified_lines.get(i).copied();

        match (orig, modi) {
            (Some(o), Some(m)) if o == m => {
                diff.push_str(&format!(" {}\n", o));
            }
            (Some(o), Some(m)) => {
                diff.push_str(&format!("-{}\n", o));
                diff.push_str(&format!("+{}\n", m));
            }
            (Some(o), None) => {
                diff.push_str(&format!("-{}\n", o));
            }
            (None, Some(m)) => {
                diff.push_str(&format!("+{}\n", m));
            }
            (None, None) => break,
        }
        i += 1;
    }

    diff
}

impl McpServer for FilesystemServer {
    fn name(&self) -> &str {
        "mcpz-filesystem"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn verbose(&self) -> bool {
        self.config.verbose
    }

    fn tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "read_file".to_string(),
                description: "Read the contents of a file. Use 'head' to read first N lines, 'tail' to read last N lines, or 'offset'/'limit' to read a specific range of lines.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        },
                        "head": {
                            "type": "integer",
                            "description": "Read only the first N lines (cannot combine with offset/limit)"
                        },
                        "tail": {
                            "type": "integer",
                            "description": "Read only the last N lines (cannot combine with offset/limit)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Line number to start reading from (1-indexed, cannot combine with head/tail)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of lines to read (cannot combine with head/tail)"
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "read_multiple_files".to_string(),
                description: "Read multiple files simultaneously. More efficient than reading one by one. Supports offset/limit for reading specific line ranges from each file.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Array of file paths to read"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Line number to start reading from in each file (1-indexed)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of lines to read from each file"
                        }
                    },
                    "required": ["paths"]
                }),
            },
            McpTool {
                name: "write_file".to_string(),
                description: "Create or overwrite a file with new content.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
            McpTool {
                name: "edit_file".to_string(),
                description: "Make line-based edits to a file. Returns a diff showing changes.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "edits": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "oldText": { "type": "string", "description": "Text to find" },
                                    "newText": { "type": "string", "description": "Text to replace with" }
                                },
                                "required": ["oldText", "newText"]
                            },
                            "description": "Array of edit operations"
                        },
                        "dryRun": {
                            "type": "boolean",
                            "description": "Preview changes without writing",
                            "default": false
                        }
                    },
                    "required": ["path", "edits"]
                }),
            },
            McpTool {
                name: "create_directory".to_string(),
                description: "Create a new directory (including parent directories).".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the directory to create"
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "list_directory".to_string(),
                description: "List contents of a directory with [FILE] and [DIR] prefixes.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the directory"
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "list_directory_with_sizes".to_string(),
                description: "List directory contents with file sizes.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the directory"
                        },
                        "sortBy": {
                            "type": "string",
                            "enum": ["name", "size"],
                            "description": "Sort by name or size",
                            "default": "name"
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "directory_tree".to_string(),
                description: "Get a recursive tree view of files and directories as JSON.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the root directory"
                        },
                        "excludePatterns": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Glob patterns to exclude",
                            "default": []
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "move_file".to_string(),
                description: "Move or rename a file or directory.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source path"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination path"
                        }
                    },
                    "required": ["source", "destination"]
                }),
            },
            McpTool {
                name: "search_files".to_string(),
                description: "Search for files matching a glob pattern.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory to search in"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern (e.g., '*.rs', '**/*.txt')"
                        },
                        "excludePatterns": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Patterns to exclude",
                            "default": []
                        }
                    },
                    "required": ["path", "pattern"]
                }),
            },
            McpTool {
                name: "get_file_info".to_string(),
                description: "Get detailed metadata about a file or directory.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file or directory"
                        }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "list_allowed_directories".to_string(),
                description: "List directories this server is allowed to access.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    fn call_tool(&self, name: &str, arguments: &serde_json::Value) -> Result<serde_json::Value> {
        match name {
            "read_file" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let head = arguments.get("head").and_then(|v| v.as_u64()).map(|n| n as usize);
                let tail = arguments.get("tail").and_then(|v| v.as_u64()).map(|n| n as usize);
                let offset = arguments.get("offset").and_then(|v| v.as_u64()).map(|n| n as usize);
                let limit = arguments.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);

                match self.read_file(path, head, tail, offset, limit) {
                    Ok(content) => Ok(text_content(&content)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "read_multiple_files" => {
                let paths: Vec<String> = arguments.get("paths")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow!("Missing 'paths' argument"))?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                let offset = arguments.get("offset").and_then(|v| v.as_u64()).map(|n| n as usize);
                let limit = arguments.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);

                match self.read_multiple_files(&paths, offset, limit) {
                    Ok(content) => Ok(text_content(&content)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "write_file" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let content = arguments.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'content' argument"))?;

                match self.write_file(path, content) {
                    Ok(msg) => Ok(text_content(&msg)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "edit_file" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let edits: Vec<EditOperation> = arguments.get("edits")
                    .ok_or_else(|| anyhow!("Missing 'edits' argument"))
                    .and_then(|v| serde_json::from_value(v.clone()).map_err(|e| anyhow!("Invalid edits: {}", e)))?;
                let dry_run = arguments.get("dryRun").and_then(|v| v.as_bool()).unwrap_or(false);

                match self.edit_file(path, edits, dry_run) {
                    Ok(diff) => Ok(text_content(&diff)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "create_directory" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

                match self.create_directory(path) {
                    Ok(msg) => Ok(text_content(&msg)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "list_directory" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

                match self.list_directory(path) {
                    Ok(content) => Ok(text_content(&content)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "list_directory_with_sizes" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let sort_by = arguments.get("sortBy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("name");

                match self.list_directory_with_sizes(path, sort_by) {
                    Ok(content) => Ok(text_content(&content)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "directory_tree" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let exclude_patterns: Vec<String> = arguments.get("excludePatterns")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                match self.directory_tree(path, &exclude_patterns) {
                    Ok(content) => Ok(text_content(&content)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "move_file" => {
                let source = arguments.get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'source' argument"))?;
                let destination = arguments.get("destination")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'destination' argument"))?;

                match self.move_file(source, destination) {
                    Ok(msg) => Ok(text_content(&msg)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "search_files" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let pattern = arguments.get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'pattern' argument"))?;
                let exclude_patterns: Vec<String> = arguments.get("excludePatterns")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                match self.search_files(path, pattern, &exclude_patterns) {
                    Ok(content) => Ok(text_content(&content)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "get_file_info" => {
                let path = arguments.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

                match self.get_file_info(path) {
                    Ok(content) => Ok(text_content(&content)),
                    Err(e) => Ok(error_content(&e.to_string())),
                }
            }
            "list_allowed_directories" => {
                Ok(text_content(&self.list_allowed_directories()))
            }
            _ => Ok(error_content(&format!("Unknown tool: {}", name))),
        }
    }
}

/// Run the filesystem MCP server
pub fn run_filesystem_server(config: FilesystemServerConfig) -> Result<()> {
    if config.verbose {
        eprintln!("[mcpz] Filesystem server configuration:");
        eprintln!("[mcpz]   Allowed directories:");
        for dir in &config.allowed_directories {
            eprintln!("[mcpz]     - {}", dir.display());
        }
    }

    let server = FilesystemServer::new(config);
    server.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_server() -> (FilesystemServer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FilesystemServerConfig::new(
            vec![temp_dir.path().to_path_buf()],
            false,
        ).unwrap();
        (FilesystemServer::new(config), temp_dir)
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_matches_glob() {
        // Simple patterns
        assert!(matches_glob("*.rs", "main.rs"));
        assert!(matches_glob("*.rs", "lib.rs"));
        assert!(!matches_glob("*.rs", "main.txt"));

        // ** patterns
        assert!(matches_glob("**/*.rs", "src/main.rs"));
        assert!(matches_glob("**/*.rs", "src/lib/mod.rs"));
        assert!(matches_glob("**/test.rs", "test.rs"));
        assert!(matches_glob("**/test.rs", "src/test.rs"));

        // Mixed patterns
        assert!(matches_glob("src/*.rs", "src/main.rs"));
        assert!(!matches_glob("src/*.rs", "lib/main.rs"));
    }

    #[test]
    fn test_read_file() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "line 1").unwrap();
        writeln!(file, "line 2").unwrap();
        writeln!(file, "line 3").unwrap();

        let content = server.read_file(file_path.to_str().unwrap(), None, None, None, None).unwrap();
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
        assert!(content.contains("line 3"));
    }

    #[test]
    fn test_read_file_head() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=10 {
            writeln!(file, "line {}", i).unwrap();
        }

        let content = server.read_file(file_path.to_str().unwrap(), Some(3), None, None, None).unwrap();
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
        assert!(content.contains("line 3"));
        assert!(!content.contains("line 4"));
    }

    #[test]
    fn test_read_file_tail() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=10 {
            writeln!(file, "line {}", i).unwrap();
        }
        drop(file); // Ensure file is flushed and closed

        let content = server.read_file(file_path.to_str().unwrap(), None, Some(3), None, None).unwrap();
        // Should contain the last 3 lines (8, 9, 10)
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() <= 3, "Expected at most 3 lines, got {}", lines.len());
        assert!(content.contains("line 10"), "Should contain line 10");
    }

    #[test]
    fn test_read_file_offset_limit() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=10 {
            writeln!(file, "line {}", i).unwrap();
        }

        // Read lines 3-5 (offset=3, limit=3)
        let content = server.read_file(file_path.to_str().unwrap(), None, None, Some(3), Some(3)).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(content.contains("line 3"));
        assert!(content.contains("line 4"));
        assert!(content.contains("line 5"));
        assert!(!content.contains("line 2"));
        assert!(!content.contains("line 6"));
    }

    #[test]
    fn test_read_file_offset_only() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=5 {
            writeln!(file, "line {}", i).unwrap();
        }

        // Read from line 3 to end
        let content = server.read_file(file_path.to_str().unwrap(), None, None, Some(3), None).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(content.contains("line 3"));
        assert!(content.contains("line 4"));
        assert!(content.contains("line 5"));
        assert!(!content.contains("line 1"));
        assert!(!content.contains("line 2"));
    }

    #[test]
    fn test_read_file_limit_only() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=10 {
            writeln!(file, "line {}", i).unwrap();
        }

        // Read first 3 lines (limit only, defaults to offset=1)
        let content = server.read_file(file_path.to_str().unwrap(), None, None, None, Some(3)).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
        assert!(content.contains("line 3"));
        assert!(!content.contains("line 4"));
    }

    #[test]
    fn test_read_file_conflicting_params() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "line 1").unwrap();

        // Cannot combine head with offset
        let result = server.read_file(file_path.to_str().unwrap(), Some(5), None, Some(1), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot combine"));

        // Cannot combine tail with limit
        let result = server.read_file(file_path.to_str().unwrap(), None, Some(5), None, Some(3));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot combine"));
    }

    #[test]
    fn test_read_multiple_files() {
        let (server, temp_dir) = create_test_server();

        // Create two test files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let mut f1 = File::create(&file1).unwrap();
        let mut f2 = File::create(&file2).unwrap();
        writeln!(f1, "file1 content").unwrap();
        writeln!(f2, "file2 content").unwrap();

        let paths = vec![
            file1.to_str().unwrap().to_string(),
            file2.to_str().unwrap().to_string(),
        ];
        let content = server.read_multiple_files(&paths, None, None).unwrap();
        assert!(content.contains("file1.txt"));
        assert!(content.contains("file1 content"));
        assert!(content.contains("file2.txt"));
        assert!(content.contains("file2 content"));
    }

    #[test]
    fn test_read_multiple_files_with_offset_limit() {
        let (server, temp_dir) = create_test_server();

        // Create two test files with multiple lines
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let mut f1 = File::create(&file1).unwrap();
        let mut f2 = File::create(&file2).unwrap();
        for i in 1..=5 {
            writeln!(f1, "file1 line {}", i).unwrap();
            writeln!(f2, "file2 line {}", i).unwrap();
        }

        let paths = vec![
            file1.to_str().unwrap().to_string(),
            file2.to_str().unwrap().to_string(),
        ];

        // Read lines 2-3 from each file
        let content = server.read_multiple_files(&paths, Some(2), Some(2)).unwrap();
        assert!(content.contains("file1 line 2"));
        assert!(content.contains("file1 line 3"));
        assert!(!content.contains("file1 line 1"));
        assert!(!content.contains("file1 line 4"));
        assert!(content.contains("file2 line 2"));
        assert!(content.contains("file2 line 3"));
        assert!(!content.contains("file2 line 1"));
        assert!(!content.contains("file2 line 4"));
    }

    #[test]
    fn test_write_file() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("new_file.txt");

        let result = server.write_file(file_path.to_str().unwrap(), "Hello, World!").unwrap();
        assert!(result.contains("Successfully wrote"));

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_create_directory() {
        let (server, temp_dir) = create_test_server();
        let dir_path = temp_dir.path().join("new_dir/nested");

        let result = server.create_directory(dir_path.to_str().unwrap()).unwrap();
        assert!(result.contains("Successfully created"));
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn test_list_directory() {
        let (server, temp_dir) = create_test_server();

        // Create some files and dirs
        File::create(temp_dir.path().join("file1.txt")).unwrap();
        File::create(temp_dir.path().join("file2.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let result = server.list_directory(temp_dir.path().to_str().unwrap()).unwrap();
        assert!(result.contains("[FILE] file1.txt"));
        assert!(result.contains("[FILE] file2.txt"));
        assert!(result.contains("[DIR] subdir"));
    }

    #[test]
    fn test_move_file() {
        let (server, temp_dir) = create_test_server();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        File::create(&source).unwrap();

        let result = server.move_file(source.to_str().unwrap(), dest.to_str().unwrap()).unwrap();
        assert!(result.contains("Successfully moved"));
        assert!(!source.exists());
        assert!(dest.exists());
    }

    #[test]
    fn test_get_file_info() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("info_test.txt");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "test content").unwrap();

        let result = server.get_file_info(file_path.to_str().unwrap()).unwrap();
        assert!(result.contains("size: 12"));
        assert!(result.contains("is_file: true"));
        assert!(result.contains("is_directory: false"));
    }

    #[test]
    fn test_search_files() {
        let (server, temp_dir) = create_test_server();

        // Create file structure
        File::create(temp_dir.path().join("test1.rs")).unwrap();
        File::create(temp_dir.path().join("test2.rs")).unwrap();
        File::create(temp_dir.path().join("other.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("src")).unwrap();
        File::create(temp_dir.path().join("src/main.rs")).unwrap();

        let result = server.search_files(temp_dir.path().to_str().unwrap(), "*.rs", &[]).unwrap();
        assert!(result.contains("test1.rs"));
        assert!(result.contains("test2.rs"));
        assert!(!result.contains("other.txt"));
    }

    #[test]
    fn test_directory_tree() {
        let (server, temp_dir) = create_test_server();

        // Create structure
        File::create(temp_dir.path().join("file.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        File::create(temp_dir.path().join("subdir/nested.txt")).unwrap();

        let result = server.directory_tree(temp_dir.path().to_str().unwrap(), &[]).unwrap();
        let tree: Vec<TreeEntry> = serde_json::from_str(&result).unwrap();

        assert!(tree.iter().any(|e| e.name == "file.txt" && e.entry_type == "file"));
        assert!(tree.iter().any(|e| e.name == "subdir" && e.entry_type == "directory"));
    }

    #[test]
    fn test_path_validation_outside_allowed() {
        let (server, _temp_dir) = create_test_server();

        // Try to access path outside allowed directory
        let result = server.read_file("/etc/passwd", None, None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Access denied"));
    }

    #[test]
    fn test_list_allowed_directories() {
        let (server, temp_dir) = create_test_server();
        let result = server.list_allowed_directories();
        assert!(result.contains("Allowed directories:"));
        assert!(result.contains(&temp_dir.path().to_string_lossy().to_string()));
    }

    #[test]
    fn test_filesystem_server_tools() {
        let (server, _temp_dir) = create_test_server();
        let tools = server.tools();

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"read_file"));
        assert!(tool_names.contains(&"write_file"));
        assert!(tool_names.contains(&"list_directory"));
        assert!(tool_names.contains(&"search_files"));
        assert!(tool_names.contains(&"create_directory"));
        assert!(tool_names.contains(&"move_file"));
        assert!(tool_names.contains(&"get_file_info"));
        assert!(tool_names.contains(&"directory_tree"));
    }

    #[test]
    fn test_filesystem_server_initialize() {
        let (server, _temp_dir) = create_test_server();
        let result = server.handle_initialize();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "mcpz-filesystem");
    }

    #[test]
    fn test_edit_file() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("edit_test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello World").unwrap();
        writeln!(file, "Goodbye World").unwrap();

        let edits = vec![
            EditOperation {
                old_text: "Hello World".to_string(),
                new_text: "Hello Rust".to_string(),
            },
        ];

        let result = server.edit_file(file_path.to_str().unwrap(), edits, false).unwrap();
        assert!(result.contains("diff"));

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Hello Rust"));
        assert!(content.contains("Goodbye World"));
    }

    #[test]
    fn test_edit_file_dry_run() {
        let (server, temp_dir) = create_test_server();
        let file_path = temp_dir.path().join("dry_run_test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Original content").unwrap();

        let edits = vec![
            EditOperation {
                old_text: "Original content".to_string(),
                new_text: "Modified content".to_string(),
            },
        ];

        let result = server.edit_file(file_path.to_str().unwrap(), edits, true).unwrap();
        assert!(result.contains("diff"));

        // File should NOT be modified in dry run
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Original content"));
        assert!(!content.contains("Modified content"));
    }
}
