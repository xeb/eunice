//! Output storage for truncated tool results with retrieval capability.
//!
//! This module stores full tool outputs and provides truncated views to the LLM,
//! with a retrieval function to access specific ranges when needed.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tempfile::TempDir;

/// Maximum size for in-memory storage (1MB)
const MAX_MEMORY_SIZE: usize = 1024 * 1024;

/// Default number of lines to show at start and end
const DEFAULT_HEAD_LINES: usize = 10;
const DEFAULT_TAIL_LINES: usize = 10;

/// Storage backend for output data
enum OutputStorage {
    /// Small outputs stored in memory
    InMemory(String),
    /// Large outputs stored in temp file
    TempFile(PathBuf),
}

/// Stored output with metadata
struct StoredOutput {
    storage: OutputStorage,
    total_lines: usize,
    #[allow(dead_code)]
    total_bytes: usize,
}

/// Session-level output store
pub struct OutputStore {
    outputs: HashMap<String, StoredOutput>,
    next_id: u64,
    temp_dir: Option<TempDir>,
}

impl OutputStore {
    /// Create a new output store
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            next_id: 1,
            temp_dir: None,
        }
    }

    /// Store output and return truncated version for LLM
    ///
    /// Returns (output_id, truncated_content) where truncated_content shows
    /// first N and last N lines if the output is large.
    pub fn store(&mut self, content: String) -> Result<(String, String)> {
        let id = format!("out_{:03}", self.next_id);
        self.next_id += 1;

        let total_bytes = content.len();
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Determine if truncation is needed
        let needs_truncation = total_lines > (DEFAULT_HEAD_LINES + DEFAULT_TAIL_LINES);

        let truncated = if needs_truncation {
            self.format_truncated(&id, &lines, total_bytes)
        } else {
            // No truncation needed, but still store for consistency
            content.clone()
        };

        // Store the full content
        let storage = if total_bytes > MAX_MEMORY_SIZE {
            // Write to temp file
            let path = self.write_temp_file(&id, &content)?;
            OutputStorage::TempFile(path)
        } else {
            OutputStorage::InMemory(content)
        };

        self.outputs.insert(
            id.clone(),
            StoredOutput {
                storage,
                total_lines,
                total_bytes,
            },
        );

        Ok((id, truncated))
    }

    /// Store shell output with exit code prominently displayed
    #[allow(dead_code)]
    pub fn store_shell_output(
        &mut self,
        exit_code: i32,
        stdout: &str,
        stderr: &str,
    ) -> Result<(String, String)> {
        // Combine stdout and stderr with labels
        let mut combined = String::new();

        if !stdout.is_empty() {
            combined.push_str(stdout);
        }

        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push_str("\n--- stderr ---\n");
            }
            combined.push_str(stderr);
        }

        if combined.is_empty() {
            combined.push_str("(no output)");
        }

        let (id, truncated) = self.store(combined)?;

        // Prepend exit code to truncated output
        let exit_status = if exit_code == 0 {
            "Exit code: 0 (OK)".to_string()
        } else {
            format!("Exit code: {} (FAILED)", exit_code)
        };

        let final_output = format!("{}\n{}", exit_status, truncated);

        Ok((id, final_output))
    }

    /// Retrieve a range of lines from stored output
    pub fn get_range(&self, id: &str, start: usize, end: Option<usize>) -> Result<String> {
        let stored = self
            .outputs
            .get(id)
            .ok_or_else(|| anyhow!("Output ID '{}' not found", id))?;

        let content = match &stored.storage {
            OutputStorage::InMemory(s) => s.clone(),
            OutputStorage::TempFile(path) => {
                let mut file = File::open(path)?;
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                content
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        let end = end.unwrap_or(start + 100).min(lines.len());
        let start = start.min(lines.len());

        if start >= end {
            return Ok(format!(
                "[No lines in range {}-{}, total lines: {}]",
                start, end, stored.total_lines
            ));
        }

        let range_lines: Vec<&str> = lines[start..end].to_vec();
        let result = format!(
            "[Output ID: {}]\nLines {}-{} of {}:\n─────────────────────────────────────────────\n{}\n─────────────────────────────────────────────",
            id,
            start + 1,  // 1-indexed for display
            end,
            stored.total_lines,
            range_lines.join("\n")
        );

        Ok(result)
    }

    /// Check if an output ID exists
    #[allow(dead_code)]
    pub fn exists(&self, id: &str) -> bool {
        self.outputs.contains_key(id)
    }

    /// Get metadata for an output
    #[allow(dead_code)]
    pub fn get_metadata(&self, id: &str) -> Option<(usize, usize)> {
        self.outputs
            .get(id)
            .map(|s| (s.total_lines, s.total_bytes))
    }

    /// Format truncated output showing first and last lines
    fn format_truncated(&self, id: &str, lines: &[&str], total_bytes: usize) -> String {
        let total_lines = lines.len();
        let head_lines: Vec<&str> = lines.iter().take(DEFAULT_HEAD_LINES).copied().collect();
        let tail_lines: Vec<&str> = lines
            .iter()
            .skip(total_lines.saturating_sub(DEFAULT_TAIL_LINES))
            .copied()
            .collect();

        let omitted = total_lines - DEFAULT_HEAD_LINES - DEFAULT_TAIL_LINES;
        let kb = total_bytes as f64 / 1024.0;

        format!(
            "[Output ID: {}]\nLines 1-{} of {} ({:.1}KB total):\n─────────────────────────────────────────────\n{}\n─────────────────────────────────────────────\n\n[... {} lines omitted ...]\n\n─────────────────────────────────────────────\n{}\n─────────────────────────────────────────────\n[Use get_output(id=\"{}\", start={}, end={}) to retrieve middle sections]",
            id,
            DEFAULT_HEAD_LINES,
            total_lines,
            kb,
            head_lines.join("\n"),
            omitted,
            tail_lines.join("\n"),
            id,
            DEFAULT_HEAD_LINES,
            DEFAULT_HEAD_LINES + 100
        )
    }

    /// Write content to a temp file
    fn write_temp_file(&mut self, id: &str, content: &str) -> Result<PathBuf> {
        // Create temp dir if not exists
        if self.temp_dir.is_none() {
            self.temp_dir = Some(TempDir::new()?);
        }

        let dir = self.temp_dir.as_ref().unwrap();
        let path = dir.path().join(format!("{}.txt", id));

        let mut file = File::create(&path)?;
        file.write_all(content.as_bytes())?;

        Ok(path)
    }
}

impl Default for OutputStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for OutputStore {
    fn drop(&mut self) {
        // TempDir handles cleanup automatically when dropped
        // But let's be explicit about clearing our state
        self.outputs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_small_output() {
        let mut store = OutputStore::new();
        let content = "line 1\nline 2\nline 3".to_string();

        let (id, truncated) = store.store(content.clone()).unwrap();

        assert_eq!(id, "out_001");
        // Small output shouldn't be truncated
        assert_eq!(truncated, content);
        assert!(store.exists(&id));
    }

    #[test]
    fn test_store_large_output() {
        let mut store = OutputStore::new();

        // Create 200 lines of output
        let lines: Vec<String> = (1..=200).map(|i| format!("line {}", i)).collect();
        let content = lines.join("\n");

        let (_id, truncated) = store.store(content).unwrap();

        // Should be truncated
        assert!(truncated.contains("[Output ID:"));
        assert!(truncated.contains("lines omitted"));
        assert!(truncated.contains("get_output"));

        // Should show first 10 lines
        assert!(truncated.contains("line 1"));
        assert!(truncated.contains("line 10"));

        // Should show last 10 lines
        assert!(truncated.contains("line 200"));
        assert!(truncated.contains("line 191"));

        // Should NOT show middle lines in truncated output
        assert!(!truncated.contains("line 50"));
    }

    #[test]
    fn test_get_range() {
        let mut store = OutputStore::new();

        let lines: Vec<String> = (1..=200).map(|i| format!("line {}", i)).collect();
        let content = lines.join("\n");

        let (id, _) = store.store(content).unwrap();

        // Get middle range
        let range = store.get_range(&id, 50, Some(60)).unwrap();

        assert!(range.contains("line 51")); // 0-indexed, so line 51 is at index 50
        assert!(range.contains("line 60"));
        assert!(!range.contains("line 61"));
    }

    #[test]
    fn test_shell_output_with_exit_code() {
        let mut store = OutputStore::new();

        let (_, output) = store
            .store_shell_output(0, "success output", "")
            .unwrap();
        assert!(output.contains("Exit code: 0 (OK)"));
        assert!(output.contains("success output"));

        let (_, output) = store
            .store_shell_output(1, "", "error message")
            .unwrap();
        assert!(output.contains("Exit code: 1 (FAILED)"));
        assert!(output.contains("error message"));
    }

    #[test]
    fn test_nonexistent_id() {
        let store = OutputStore::new();
        let result = store.get_range("nonexistent", 0, Some(10));
        assert!(result.is_err());
    }

    #[test]
    fn test_sequential_ids() {
        let mut store = OutputStore::new();

        let (id1, _) = store.store("first".to_string()).unwrap();
        let (id2, _) = store.store("second".to_string()).unwrap();
        let (id3, _) = store.store("third".to_string()).unwrap();

        assert_eq!(id1, "out_001");
        assert_eq!(id2, "out_002");
        assert_eq!(id3, "out_003");
    }
}
