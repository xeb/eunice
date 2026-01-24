use crate::mcpz::servers::browser::BrowserResult;

/// Check if stdout is a TTY (interactive terminal)
pub fn is_tty() -> bool {
    atty::is(atty::Stream::Stdout)
}

/// Format a BrowserResult for output
pub fn format_result(result: &BrowserResult, json_mode: bool) -> String {
    if json_mode || !is_tty() {
        serde_json::to_string(result).unwrap_or_else(|_| format!("{{\"error\": \"serialization failed\"}}"))
    } else {
        format_human(result)
    }
}

/// Human-readable formatting
fn format_human(result: &BrowserResult) -> String {
    if !result.success {
        return format!("Error: {}", result.message);
    }

    // Special formatting for known data shapes
    if let Some(ref data) = result.data {
        // Tabs list
        if let Some(tabs) = data.get("tabs").and_then(|t| t.as_array()) {
            let mut out = String::new();
            out.push_str(&format!("{:<14} {:<50} {}\n", "ID", "URL", "TITLE"));
            for tab in tabs {
                let id = tab.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let url = tab.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let title = tab.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let url_display = if url.len() > 50 { &url[..47] } else { url };
                out.push_str(&format!("{:<14} {:<50} {}\n", id, url_display, title));
            }
            return out.trim_end().to_string();
        }

        // HTML content
        if let Some(html) = data.get("html").and_then(|h| h.as_str()) {
            return html.to_string();
        }

        // Markdown content
        if let Some(md) = data.get("markdown").and_then(|m| m.as_str()) {
            return md.to_string();
        }

        // Cookies
        if let Some(cookies) = data.get("cookies").and_then(|c| c.as_array()) {
            let mut out = String::new();
            out.push_str(&format!("{:<20} {:<30} {}\n", "NAME", "DOMAIN", "VALUE"));
            for cookie in cookies {
                let name = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let domain = cookie.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let val_display = if value.len() > 40 { &value[..37] } else { value };
                out.push_str(&format!("{:<20} {:<30} {}...\n", name, domain, val_display));
            }
            return out.trim_end().to_string();
        }

        // Script result
        if let Some(result_val) = data.get("result") {
            return serde_json::to_string_pretty(result_val).unwrap_or_else(|_| result_val.to_string());
        }
    }

    result.message.clone()
}

/// Format error for output
pub fn format_error(msg: &str, json_mode: bool) -> String {
    if json_mode || !is_tty() {
        serde_json::json!({"success": false, "message": msg}).to_string()
    } else {
        format!("Error: {}", msg)
    }
}
