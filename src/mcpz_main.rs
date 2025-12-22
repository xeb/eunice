//! mcpz binary entry point
//!
//! This is the entry point for the mcpz binary, which is part of the eunice crate.
//! mcpz is a runtime MCP router tool for running MCP servers via npx, uvx, or cargo.

mod mcpz;

fn main() -> anyhow::Result<()> {
    mcpz::run()
}
