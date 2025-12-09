# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "fastmcp",
# ]
# ///
"""
Sample MCP server using FastMCP with Streamable HTTP transport.

Run with:
    uv run sample_mcp_server.py

Test with eunice:
    eunice --config eunice.toml "reverse the string 'hello world'"
"""

from fastmcp import FastMCP

mcp = FastMCP("Sample MCP Server")


@mcp.tool()
def reverse_echo(text: str) -> str:
    """Reverses the input string and returns it.

    Args:
        text: The string to reverse

    Returns:
        The reversed string
    """
    return text[::-1]


if __name__ == "__main__":
    mcp.run(transport="streamable-http", host="0.0.0.0", port=8773)
