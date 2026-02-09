use crate::models::Tool;
use crate::tools::make_tool;
use anyhow::{Context, Result};
use std::path::Path;

/// Write tool for writing content to files
pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }

    pub fn get_spec(&self) -> Tool {
        make_tool(
            "Write",
            "Write content to a file. Creates parent directories if needed. Overwrites existing files.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or relative path to the file"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        )
    }

    pub fn execute(&self, args: serde_json::Value) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        let path = Path::new(path_str);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }
        }

        // Check if path is a directory
        if path.exists() && path.is_dir() {
            return Err(anyhow::anyhow!("Cannot write to a directory: {}", path_str));
        }

        // Write the file
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write file: {}", path_str))?;

        let bytes = content.len();
        let lines = content.lines().count();

        Ok(format!("Wrote {} bytes ({} lines) to {}", bytes, lines, path_str))
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_tool_spec() {
        let tool = WriteTool::new();
        let spec = tool.get_spec();
        assert_eq!(spec.function.name, "Write");
        assert!(spec.function.description.contains("Write"));
    }

    #[test]
    fn test_write_new_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Hello, World!"
        });

        let result = tool.execute(args).unwrap();
        assert!(result.contains("13 bytes"));
        assert!(result.contains("1 lines"));

        // Verify file was written
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a").join("b").join("c").join("test.txt");

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "nested content"
        });

        let result = tool.execute(args).unwrap();
        assert!(result.contains("bytes"));
        assert!(path.exists());
    }

    #[test]
    fn test_write_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");

        // Write initial content
        std::fs::write(&path, "original").unwrap();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "new content"
        });

        tool.execute(args).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_write_missing_path() {
        let tool = WriteTool::new();
        let args = serde_json::json!({"content": "test"});
        let result = tool.execute(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_missing_content() {
        let tool = WriteTool::new();
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let result = tool.execute(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_multiline_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("multiline.txt");

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "line1\nline2\nline3"
        });

        let result = tool.execute(args).unwrap();
        assert!(result.contains("3 lines"));
    }
}
