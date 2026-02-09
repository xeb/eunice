use crate::models::Tool;
use crate::tools::make_tool;
use anyhow::{Context, Result};
use std::path::Path;

/// Read tool for reading file contents
pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }

    pub fn get_spec(&self) -> Tool {
        make_tool(
            "Read",
            "Read the contents of a file. Returns the file content as a string. For binary files, returns a message indicating the file type.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or relative path to the file"
                    }
                },
                "required": ["path"]
            }),
        )
    }

    pub fn execute(&self, args: serde_json::Value) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let path = Path::new(path_str);

        // Check if file exists
        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", path_str));
        }

        // Check if it's a directory
        if path.is_dir() {
            return Err(anyhow::anyhow!("Path is a directory, not a file: {}", path_str));
        }

        // Read the file
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file: {}", path_str))?;

        // Check if it's valid UTF-8 (text file)
        match String::from_utf8(content.clone()) {
            Ok(text) => Ok(text),
            Err(_) => {
                // Binary file - return info about it
                let size = content.len();
                let extension = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown");

                // Try to guess the file type from magic bytes
                let file_type = detect_binary_type(&content, extension);

                Ok(format!(
                    "[Binary file: {} bytes, type: {}]\nUse the Bash tool to process this file with appropriate commands.",
                    size, file_type
                ))
            }
        }
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect binary file type from magic bytes and extension
fn detect_binary_type(content: &[u8], extension: &str) -> &'static str {
    // Check magic bytes
    if content.len() >= 4 {
        // PNG
        if content.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return "PNG image";
        }
        // JPEG
        if content.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return "JPEG image";
        }
        // GIF
        if content.starts_with(b"GIF8") {
            return "GIF image";
        }
        // PDF
        if content.starts_with(b"%PDF") {
            return "PDF document";
        }
        // ZIP/DOCX/XLSX/etc
        if content.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
            return match extension {
                "docx" => "Microsoft Word document",
                "xlsx" => "Microsoft Excel spreadsheet",
                "pptx" => "Microsoft PowerPoint presentation",
                "zip" => "ZIP archive",
                _ => "ZIP-based archive",
            };
        }
        // ELF binary
        if content.starts_with(&[0x7F, 0x45, 0x4C, 0x46]) {
            return "ELF executable";
        }
        // Mach-O binary
        if content.starts_with(&[0xFE, 0xED, 0xFA, 0xCE])
            || content.starts_with(&[0xFE, 0xED, 0xFA, 0xCF])
            || content.starts_with(&[0xCE, 0xFA, 0xED, 0xFE])
            || content.starts_with(&[0xCF, 0xFA, 0xED, 0xFE])
        {
            return "Mach-O executable";
        }
    }

    // Fall back to extension-based detection
    match extension {
        "png" => "PNG image",
        "jpg" | "jpeg" => "JPEG image",
        "gif" => "GIF image",
        "webp" => "WebP image",
        "pdf" => "PDF document",
        "mp3" => "MP3 audio",
        "mp4" => "MP4 video",
        "wav" => "WAV audio",
        "exe" => "Windows executable",
        "so" | "dylib" => "Shared library",
        "a" => "Static library",
        "tar" => "TAR archive",
        "gz" | "gzip" => "Gzip compressed",
        "bz2" => "Bzip2 compressed",
        "xz" => "XZ compressed",
        _ => "binary",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_tool_spec() {
        let tool = ReadTool::new();
        let spec = tool.get_spec();
        assert_eq!(spec.function.name, "Read");
        assert!(spec.function.description.contains("file"));
    }

    #[test]
    fn test_read_text_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "Hello, World!").unwrap();

        let tool = ReadTool::new();
        let args = serde_json::json!({"path": file.path().to_str().unwrap()});
        let result = tool.execute(args).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_read_missing_file() {
        let tool = ReadTool::new();
        let args = serde_json::json!({"path": "/nonexistent/file.txt"});
        let result = tool.execute(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_read_missing_path_param() {
        let tool = ReadTool::new();
        let args = serde_json::json!({});
        let result = tool.execute(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_binary_file() {
        let mut file = NamedTempFile::with_suffix(".png").unwrap();
        // Write PNG magic bytes
        file.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();

        let tool = ReadTool::new();
        let args = serde_json::json!({"path": file.path().to_str().unwrap()});
        let result = tool.execute(args).unwrap();
        assert!(result.contains("Binary file"));
        assert!(result.contains("PNG image"));
    }

    #[test]
    fn test_detect_binary_types() {
        assert_eq!(detect_binary_type(&[0x89, 0x50, 0x4E, 0x47], ""), "PNG image");
        assert_eq!(detect_binary_type(&[0xFF, 0xD8, 0xFF, 0xE0], ""), "JPEG image");
        assert_eq!(detect_binary_type(b"%PDF-1.4", ""), "PDF document");
        assert_eq!(detect_binary_type(&[0x50, 0x4B, 0x03, 0x04], "zip"), "ZIP archive");
        assert_eq!(detect_binary_type(&[0x50, 0x4B, 0x03, 0x04], "docx"), "Microsoft Word document");
    }
}
