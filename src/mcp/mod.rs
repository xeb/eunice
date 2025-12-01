pub mod http_server;
pub mod manager;
pub mod server;

pub use manager::McpManager;

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
}
