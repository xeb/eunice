//! Centralized TUI theme: palette + small render helpers (Claude-style).
//!
//! Colors use *relative* SGR where possible (e.g. reverse video for the user bar) so they
//! adapt to the user's terminal theme instead of hardcoding a background.
#![allow(dead_code)] // full palette is kept for reuse; not every color is wired in yet.

use crossterm::terminal;

// 256-color SGR foreground codes + attributes.
pub const ACCENT: &str = "\x1b[38;5;209m"; // warm coral — thinking / status
pub const PURPLE: &str = "\x1b[38;5;141m"; // mode label / brand
pub const PINK: &str = "\x1b[38;5;211m"; // footer hint
pub const BLUE: &str = "\x1b[34m";
pub const BRIGHT_BLUE: &str = "\x1b[94m";
pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const REVERSE: &str = "\x1b[7m";
pub const RESET: &str = "\x1b[0m";

/// Spinner glyphs (asterisk / sparkle cycle, Claude-style).
pub const SPINNER: &[&str] = &["✻", "✶", "✺", "✹", "✷"];

/// Terminal width (columns), defaulting to 80 when undetectable.
pub fn term_cols() -> usize {
    terminal::size().map(|(w, _)| w as usize).unwrap_or(80)
}

/// A full-width dim horizontal rule, optionally with a right-aligned label (the input-box top).
pub fn rule_with_width(cols: usize, label: Option<&str>) -> String {
    let cols = cols.max(1);
    match label {
        Some(l) => {
            let label_disp = format!(" {} ", l);
            let label_len = label_disp.chars().count();
            let dashes = cols.saturating_sub(label_len).max(1);
            format!(
                "{DIM}{}{RESET}{PURPLE}{}{RESET}",
                "─".repeat(dashes),
                label_disp
            )
        }
        None => format!("{DIM}{}{RESET}", "─".repeat(cols)),
    }
}

/// Convenience: a rule sized to the current terminal.
pub fn rule(label: Option<&str>) -> String {
    rule_with_width(term_cols(), label)
}

/// Render a submitted user turn as a full-width inverse (black) bold bar.
pub fn user_bar_with_width(cols: usize, text: &str) -> String {
    let cols = cols.max(1);
    let inner = cols.saturating_sub(1); // reserve one leading space
    let truncated: String = text.chars().take(inner).collect();
    let used = truncated.chars().count() + 1; // + leading space
    let pad = cols.saturating_sub(used);
    format!(
        "{REVERSE}{BOLD} {}{}{RESET}",
        truncated,
        " ".repeat(pad)
    )
}

/// Convenience: a user bar sized to the current terminal.
pub fn user_bar(text: &str) -> String {
    user_bar_with_width(term_cols(), text)
}

/// Footer hint line (accent/pink), e.g. keybindings.
pub fn footer(hint: &str) -> String {
    format!("{PINK}▸▸ {}{RESET}", hint)
}

/// The thinking/status line: accent glyph + verb, optional elapsed/effort.
pub fn thinking_line(glyph: &str, verb: &str, elapsed_secs: Option<u64>, effort: Option<&str>) -> String {
    let mut meta: Vec<String> = Vec::new();
    if let Some(s) = elapsed_secs {
        meta.push(format!("{}s", s));
    }
    if let Some(e) = effort {
        meta.push(format!("{} effort", e));
    }
    let suffix = if meta.is_empty() {
        String::new()
    } else {
        format!(" {DIM}({}){RESET}{ACCENT}", meta.join(" · "))
    };
    format!("{ACCENT}{} {}…{}{RESET}", glyph, verb, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible_len(s: &str) -> usize {
        // Strip SGR escape sequences, then count chars.
        let mut out = String::new();
        let mut in_esc = false;
        for c in s.chars() {
            if in_esc {
                if c == 'm' {
                    in_esc = false;
                }
            } else if c == '\x1b' {
                in_esc = true;
            } else {
                out.push(c);
            }
        }
        out.chars().count()
    }

    #[test]
    fn user_bar_pads_to_full_width() {
        let s = user_bar_with_width(40, "hello");
        assert_eq!(visible_len(&s), 40, "bar must span the full width");
        assert!(s.contains(REVERSE) && s.contains(BOLD) && s.ends_with(RESET));
        assert!(s.contains("hello"));
    }

    #[test]
    fn user_bar_truncates_long_text() {
        let long = "x".repeat(200);
        let s = user_bar_with_width(40, &long);
        assert_eq!(visible_len(&s), 40);
    }

    #[test]
    fn rule_without_label_fills_width() {
        let s = rule_with_width(30, None);
        assert_eq!(visible_len(&s), 30);
    }

    #[test]
    fn rule_with_label_fills_width() {
        let s = rule_with_width(30, Some("eunice"));
        assert_eq!(visible_len(&s), 30, "rule + label must total the width");
        assert!(s.contains("eunice"));
    }

    #[test]
    fn thinking_line_formats_meta() {
        let s = thinking_line("✻", "Thinking", Some(12), Some("xhigh"));
        assert!(s.contains("Thinking…"));
        assert!(s.contains("12s"));
        assert!(s.contains("xhigh effort"));
        let bare = thinking_line("✻", "Thinking", None, None);
        assert!(bare.contains("Thinking…"));
        assert!(!bare.contains('('));
    }
}
