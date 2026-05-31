//! Bottom-anchored, vertically-expanding framed input editor (Claude-style).
//!
//! The box is **anchored to the bottom of the screen** and its geometry is recomputed from the
//! terminal size on every redraw (we never cache a row that could go stale after a scroll). Space
//! for the box is reserved by scrolling the screen up first; as the buffer wraps we reserve more.
//! Built on the raw-mode editing approach proven in `interactive.rs`.

use crate::theme;
use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{Clear, ClearType};
use crossterm::ExecutableCommand;
use std::io::{self, Write};
use std::time::Duration;

/// Visible width of the `› ` prompt.
const PROMPT_COLS: usize = 2;

/// Non-buffer rows in the box: top rule + bottom rule + footer.
const CHROME_ROWS: u16 = 3;

/// Slash commands offered for Tab-completion.
const SLASH_COMMANDS: &[&str] = &["/help", "/clear", "/status", "/quit"];

pub enum LineResult {
    Line(String),
    Eof,
    Interrupted,
}

/// Number of buffer (text) rows needed for `buf_chars` characters at `cols` width.
fn buffer_rows(buf_chars: usize, cols: usize) -> usize {
    let cols = cols.max(1);
    let total = PROMPT_COLS + buf_chars;
    ((total + cols - 1) / cols).max(1)
}

/// Total box height (rows) for a buffer of `buf_chars` characters.
fn box_height(buf_chars: usize, cols: usize) -> u16 {
    buffer_rows(buf_chars, cols) as u16 + CHROME_ROWS
}

fn term_rows() -> u16 {
    crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24)
}

fn first_match(prefix: &str) -> Option<&'static str> {
    if !prefix.starts_with('/') {
        return None;
    }
    SLASH_COMMANDS
        .iter()
        .find(|c| c.starts_with(prefix) && **c != prefix)
        .copied()
}

/// Reserve at least `need` blank rows at the bottom by scrolling the screen up. Returns the new
/// reserved-row count (never shrinks within a single edit session).
fn ensure_reserved(stdout: &mut io::Stdout, reserved: u16, need: u16) -> io::Result<u16> {
    if need > reserved {
        let rows = term_rows();
        stdout.execute(MoveTo(0, rows.saturating_sub(1)))?;
        for _ in 0..(need - reserved) {
            write!(stdout, "\r\n")?;
        }
        Ok(need)
    } else {
        Ok(reserved)
    }
}

/// Draw the box anchored at the bottom of the screen, then position the cursor in the buffer.
fn draw_box(
    stdout: &mut io::Stdout,
    reserved: u16,
    buffer: &str,
    cursor_pos: usize,
    label: &str,
    footer: &str,
) -> io::Result<()> {
    let cols = theme::term_cols();
    let rows = term_rows();
    let buf_chars = buffer.chars().count();
    let brows = buffer_rows(buf_chars, cols) as u16;
    let h = brows + CHROME_ROWS;
    let region_top = rows.saturating_sub(reserved);
    let box_top = rows.saturating_sub(h);

    // Clear the whole reserved region, then draw the box at the bottom.
    stdout.execute(MoveTo(0, region_top))?;
    stdout.execute(Clear(ClearType::FromCursorDown))?;

    stdout.execute(MoveTo(0, box_top))?;
    write!(stdout, "{}", theme::rule_with_width(cols, Some(label)))?;

    stdout.execute(MoveTo(0, box_top + 1))?;
    write!(stdout, "{}› {}{}", theme::PURPLE, theme::RESET, buffer)?;

    stdout.execute(MoveTo(0, box_top + 1 + brows))?;
    write!(stdout, "{}", theme::rule_with_width(cols, None))?;

    stdout.execute(MoveTo(0, box_top + 2 + brows))?;
    write!(stdout, "{}", theme::footer(footer))?;

    // Position the cursor within the buffer.
    let cols = cols.max(1);
    let total = PROMPT_COLS + cursor_pos;
    let crow = box_top + 1 + (total / cols) as u16;
    let ccol = (total % cols) as u16;
    stdout.execute(MoveTo(ccol, crow))?;
    stdout.flush()?;
    Ok(())
}

/// Erase the reserved region (used on submit/exit), leaving the cursor at its top so the caller
/// can print in its place.
fn erase_region(stdout: &mut io::Stdout, reserved: u16) -> io::Result<()> {
    let rows = term_rows();
    stdout.execute(MoveTo(0, rows.saturating_sub(reserved)))?;
    stdout.execute(Clear(ClearType::FromCursorDown))?;
    stdout.flush()?;
    Ok(())
}

/// Read one line of input inside the framed, expanding box. Caller owns raw mode.
pub fn read_line_framed(history: &[String], label: &str, footer: &str) -> io::Result<LineResult> {
    let mut stdout = io::stdout();
    let cols = theme::term_cols();

    // Reserve initial space for an empty box at the bottom (scrolls the screen up).
    let mut reserved = box_height(0, cols);
    {
        let rows = term_rows();
        stdout.execute(MoveTo(0, rows.saturating_sub(1)))?;
        for _ in 0..reserved {
            write!(stdout, "\r\n")?;
        }
        stdout.flush()?;
    }

    let mut buffer = String::new();
    let mut cursor_pos: usize = 0;
    let mut history_pos: usize = history.len();
    let mut saved_input = String::new();

    draw_box(&mut stdout, reserved, &buffer, cursor_pos, label, footer)?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                let mut redraw = false;
                match (code, modifiers) {
                    (KeyCode::Enter, _) => {
                        erase_region(&mut stdout, reserved)?;
                        return Ok(LineResult::Line(buffer));
                    }
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                        erase_region(&mut stdout, reserved)?;
                        return Ok(LineResult::Interrupted);
                    }
                    (KeyCode::Char('d'), m) if m.contains(KeyModifiers::CONTROL) => {
                        if buffer.is_empty() {
                            erase_region(&mut stdout, reserved)?;
                            return Ok(LineResult::Eof);
                        }
                    }
                    // Esc clears the current input (never dismisses the box).
                    (KeyCode::Esc, _) => {
                        buffer.clear();
                        cursor_pos = 0;
                        redraw = true;
                    }
                    (KeyCode::Tab, _) => {
                        if let Some(m) = first_match(&buffer) {
                            buffer = m.to_string();
                            cursor_pos = buffer.chars().count();
                            redraw = true;
                        }
                    }
                    (KeyCode::Char('a'), m) if m.contains(KeyModifiers::CONTROL) => {
                        cursor_pos = 0;
                        redraw = true;
                    }
                    (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
                        cursor_pos = buffer.chars().count();
                        redraw = true;
                    }
                    (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
                        buffer.clear();
                        cursor_pos = 0;
                        redraw = true;
                    }
                    (KeyCode::Char('w'), m) if m.contains(KeyModifiers::CONTROL) => {
                        let chars: Vec<char> = buffer.chars().collect();
                        let mut new_pos = cursor_pos;
                        while new_pos > 0 && chars.get(new_pos - 1) == Some(&' ') {
                            new_pos -= 1;
                        }
                        while new_pos > 0 && chars.get(new_pos - 1) != Some(&' ') {
                            new_pos -= 1;
                        }
                        buffer = chars[..new_pos].iter().chain(chars[cursor_pos..].iter()).collect();
                        cursor_pos = new_pos;
                        redraw = true;
                    }
                    (KeyCode::Char(c), _) => {
                        let byte_idx = buffer.char_indices().nth(cursor_pos).map(|(i, _)| i).unwrap_or(buffer.len());
                        buffer.insert(byte_idx, c);
                        cursor_pos += 1;
                        redraw = true;
                    }
                    (KeyCode::Backspace, _) => {
                        if cursor_pos > 0 {
                            let byte_idx = buffer.char_indices().nth(cursor_pos - 1).map(|(i, _)| i).unwrap_or(0);
                            buffer.remove(byte_idx);
                            cursor_pos -= 1;
                            redraw = true;
                        }
                    }
                    (KeyCode::Delete, _) => {
                        if cursor_pos < buffer.chars().count() {
                            let byte_idx = buffer.char_indices().nth(cursor_pos).map(|(i, _)| i).unwrap_or(buffer.len());
                            buffer.remove(byte_idx);
                            redraw = true;
                        }
                    }
                    (KeyCode::Left, _) => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                            redraw = true;
                        }
                    }
                    (KeyCode::Right, _) => {
                        if cursor_pos < buffer.chars().count() {
                            cursor_pos += 1;
                            redraw = true;
                        }
                    }
                    (KeyCode::Home, _) => {
                        cursor_pos = 0;
                        redraw = true;
                    }
                    (KeyCode::End, _) => {
                        cursor_pos = buffer.chars().count();
                        redraw = true;
                    }
                    (KeyCode::Up, _) => {
                        if !history.is_empty() && history_pos > 0 {
                            if history_pos == history.len() {
                                saved_input = buffer.clone();
                            }
                            history_pos -= 1;
                            buffer = history[history_pos].clone();
                            cursor_pos = buffer.chars().count();
                            redraw = true;
                        }
                    }
                    (KeyCode::Down, _) => {
                        if history_pos < history.len() {
                            history_pos += 1;
                            buffer = if history_pos == history.len() {
                                saved_input.clone()
                            } else {
                                history[history_pos].clone()
                            };
                            cursor_pos = buffer.chars().count();
                            redraw = true;
                        }
                    }
                    _ => {}
                }

                if redraw {
                    let need = box_height(buffer.chars().count(), theme::term_cols());
                    reserved = ensure_reserved(&mut stdout, reserved, need)?;
                    draw_box(&mut stdout, reserved, &buffer, cursor_pos, label, footer)?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_rows_grows_with_text() {
        assert_eq!(buffer_rows(0, 80), 1);
        assert_eq!(buffer_rows(10, 80), 1);
        assert_eq!(buffer_rows(78, 80), 1); // 2 + 78 == 80 -> one row
        assert_eq!(buffer_rows(79, 80), 2); // wraps to a second row
        assert_eq!(buffer_rows(158, 80), 2);
        assert_eq!(buffer_rows(159, 80), 3);
        assert_eq!(buffer_rows(200, 80), 3); // ceil((2+200)/80) == 3
    }

    #[test]
    fn buffer_rows_handles_tiny_width() {
        assert!(buffer_rows(10, 0) >= 1);
        assert!(buffer_rows(10, 1) >= 1);
    }

    #[test]
    fn box_height_is_buffer_plus_chrome() {
        assert_eq!(box_height(0, 80), 4); // 1 buffer + 3 chrome
        assert_eq!(box_height(79, 80), 5); // 2 buffer + 3 chrome
    }

    #[test]
    fn first_match_completes_slash_commands() {
        assert_eq!(first_match("/he"), Some("/help"));
        assert_eq!(first_match("/cl"), Some("/clear"));
        assert_eq!(first_match("hello"), None);
        assert_eq!(first_match("/help"), None);
    }
}
