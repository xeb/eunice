pub mod http_server;
pub mod manager;
pub mod server;

pub use manager::McpManager;

/// Sanitize a tool name to be compatible with all AI providers (especially Gemini).
/// Replaces invalid characters with underscores. Valid characters are: a-z, A-Z, 0-9, _
/// Returns the sanitized name and whether it was modified.
pub fn sanitize_tool_name(name: &str) -> (String, bool) {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Ensure name doesn't start with a digit
    let sanitized = if sanitized.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        format!("_{}", sanitized)
    } else {
        sanitized
    };

    let modified = sanitized != name;
    (sanitized, modified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_tool_name_no_change() {
        let (name, modified) = sanitize_tool_name("my_tool_name");
        assert_eq!(name, "my_tool_name");
        assert!(!modified);
    }

    #[test]
    fn test_sanitize_tool_name_with_hyphen() {
        let (name, modified) = sanitize_tool_name("my-tool-name");
        assert_eq!(name, "my_tool_name");
        assert!(modified);
    }

    #[test]
    fn test_sanitize_tool_name_with_dot() {
        let (name, modified) = sanitize_tool_name("server.list_files");
        assert_eq!(name, "server_list_files");
        assert!(modified);
    }

    #[test]
    fn test_sanitize_tool_name_starting_with_digit() {
        let (name, modified) = sanitize_tool_name("123tool");
        assert_eq!(name, "_123tool");
        assert!(modified);
    }

    #[test]
    fn test_sanitize_tool_name_with_special_chars() {
        let (name, modified) = sanitize_tool_name("tool@name#1");
        assert_eq!(name, "tool_name_1");
        assert!(modified);
    }
}
