pub mod http_server;
pub mod manager;
pub mod server;

pub use manager::McpManager;

/// Maximum tool name length supported by Gemini
pub const MAX_TOOL_NAME_LENGTH: usize = 64;

/// Check if a tool name matches a pattern (supports * wildcard)
/// Examples:
///   - "eng_file_read" matches "eng_file_read" (exact)
///   - "eng_file_read" matches "eng_*" (prefix wildcard)
///   - "eng_file_read" matches "*_read" (suffix wildcard)
///   - "eng_file_read" matches "*" (match all)
///   - "eng_file_read" matches "eng_*_read" (middle wildcard)
pub fn tool_matches_pattern(tool_name: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') {
        // Exact match
        return tool_name == pattern;
    }

    // Split pattern by * and check if parts match in order
    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 2 {
        // Single wildcard: prefix*, *suffix, or *
        let (prefix, suffix) = (parts[0], parts[1]);
        if prefix.is_empty() {
            // *suffix
            tool_name.ends_with(suffix)
        } else if suffix.is_empty() {
            // prefix*
            tool_name.starts_with(prefix)
        } else {
            // prefix*suffix
            tool_name.starts_with(prefix) && tool_name.ends_with(suffix) && tool_name.len() >= prefix.len() + suffix.len()
        }
    } else {
        // Multiple wildcards - check parts appear in order
        let mut remaining = tool_name;

        // First part must be prefix (unless empty)
        if !parts[0].is_empty() {
            if !remaining.starts_with(parts[0]) {
                return false;
            }
            remaining = &remaining[parts[0].len()..];
        }

        // Middle parts must appear in order
        for part in &parts[1..parts.len()-1] {
            if part.is_empty() {
                continue;
            }
            if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }

        // Last part must be suffix (unless empty)
        let last = parts[parts.len() - 1];
        if !last.is_empty() {
            remaining.ends_with(last)
        } else {
            true
        }
    }
}

/// Check if a tool name exceeds the maximum length and print a warning if so.
/// Returns true if a warning was printed.
pub fn warn_if_tool_name_too_long(tool_name: &str, server_name: &str) -> bool {
    if tool_name.len() > MAX_TOOL_NAME_LENGTH {
        eprintln!(
            "  ⚠️  Warning: Tool '{}' ({} chars) exceeds Gemini's {} char limit.",
            tool_name,
            tool_name.len(),
            MAX_TOOL_NAME_LENGTH
        );
        eprintln!(
            "      Consider renaming server '{}' to something shorter in your config.",
            server_name
        );
        true
    } else {
        false
    }
}

/// Sanitize a JSON schema by removing extension fields (x-*) that providers like Gemini don't support.
/// This recursively walks the schema and removes any keys starting with "x-".
pub fn sanitize_schema(schema: &mut serde_json::Value) {
    match schema {
        serde_json::Value::Object(map) => {
            // Remove keys starting with "x-"
            let keys_to_remove: Vec<String> = map
                .keys()
                .filter(|k| k.starts_with("x-"))
                .cloned()
                .collect();
            for key in keys_to_remove {
                map.remove(&key);
            }
            // Recursively sanitize remaining values
            for value in map.values_mut() {
                sanitize_schema(value);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                sanitize_schema(item);
            }
        }
        _ => {}
    }
}

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

    #[test]
    fn test_sanitize_schema_removes_x_fields() {
        let mut schema = serde_json::json!({
            "type": "object",
            "properties": {
                "field1": {
                    "type": "string",
                    "x-google-enum-descriptions": ["desc1", "desc2"]
                }
            }
        });
        sanitize_schema(&mut schema);
        assert!(schema["properties"]["field1"].get("x-google-enum-descriptions").is_none());
        assert_eq!(schema["properties"]["field1"]["type"], "string");
    }

    #[test]
    fn test_sanitize_schema_nested() {
        let mut schema = serde_json::json!({
            "type": "object",
            "x-custom": "remove me",
            "properties": {
                "nested": {
                    "type": "object",
                    "x-google-enum-deprecated": ["old"],
                    "properties": {
                        "deep": {
                            "x-another": "also remove"
                        }
                    }
                }
            }
        });
        sanitize_schema(&mut schema);
        assert!(schema.get("x-custom").is_none());
        assert!(schema["properties"]["nested"].get("x-google-enum-deprecated").is_none());
        assert!(schema["properties"]["nested"]["properties"]["deep"].get("x-another").is_none());
    }

    #[test]
    fn test_tool_matches_pattern_exact() {
        assert!(tool_matches_pattern("eng_file_read", "eng_file_read"));
        assert!(!tool_matches_pattern("eng_file_read", "eng_file_write"));
    }

    #[test]
    fn test_tool_matches_pattern_prefix_wildcard() {
        assert!(tool_matches_pattern("eng_file_read", "eng_*"));
        assert!(tool_matches_pattern("eng_file_write", "eng_*"));
        assert!(!tool_matches_pattern("other_file_read", "eng_*"));
    }

    #[test]
    fn test_tool_matches_pattern_suffix_wildcard() {
        assert!(tool_matches_pattern("eng_file_read", "*_read"));
        assert!(tool_matches_pattern("other_file_read", "*_read"));
        assert!(!tool_matches_pattern("eng_file_write", "*_read"));
    }

    #[test]
    fn test_tool_matches_pattern_match_all() {
        assert!(tool_matches_pattern("eng_file_read", "*"));
        assert!(tool_matches_pattern("anything", "*"));
    }

    #[test]
    fn test_tool_matches_pattern_middle_wildcard() {
        assert!(tool_matches_pattern("eng_file_read", "eng_*_read"));
        assert!(tool_matches_pattern("eng_something_read", "eng_*_read"));
        assert!(!tool_matches_pattern("eng_file_write", "eng_*_read"));
        assert!(!tool_matches_pattern("other_file_read", "eng_*_read"));
    }
}
