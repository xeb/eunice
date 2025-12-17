//! TUI mode using r3bl_tui's readline_async for coordinated terminal output.
//!
//! This module provides a proper TUI experience using:
//! - ReadlineAsyncContext for async user input
//! - SharedWriter for coordinated output from concurrent tasks
//! - Spinner for thinking indicators
//!
//! This eliminates the jumbled output issues by properly coordinating
//! all terminal writes through the shared writer.

mod app;

pub use app::run_tui_mode;
