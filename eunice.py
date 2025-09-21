#!/usr/bin/env python3
# /// script
# requires-python = ">=3.13"
# dependencies = [
#     "openai",
#     "rich",
# ]
# ///
"""
eunice - Agentic CLI runner
Usage: eunice [--model=MODEL] [--prompt=PROMPT] [prompt]
"""

__version__ = "1.1.2"

import argparse
import asyncio
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path
from typing import Any, Dict, List, Optional, Union, AsyncGenerator


import openai
from openai import OpenAI

from rich.console import Console
from rich.panel import Panel
from rich import print as rich_print

# Initialize Rich console
console = Console()

def print_tool_invocation(tool_name: str, tool_args: Dict[str, Any], silent: bool = False) -> None:
    """Print a formatted tool invocation with light blue color and framing."""
    if silent:
        return

    args_str = json.dumps(tool_args, indent=None, separators=(',', ':'))
    content = f"🔧 {tool_name}({args_str})"

    console.print(Panel(content, border_style="bright_blue", title="Tool Call"))

def print_tool_result(result: str, output_limit: int = 50, silent: bool = False) -> None:
    """Print a formatted tool result with green color and framing."""
    if silent:
        return

    original_length = len(result)
    # Apply truncation if limit > 0
    if output_limit > 0 and len(result) > output_limit:
        result = result[:output_limit]
        truncated_chars = original_length - output_limit
        result += f"\n...{truncated_chars} characters truncated"

    console.print(Panel(result, border_style="green", title="Result"))

def print_model_info(model: str, provider: str, silent: bool = False) -> None:
    """Print a formatted model information window with light yellow color and framing."""
    if silent:
        return

    content = f"🤖 Model: {model} ({provider})"
    console.print(Panel(content, border_style="yellow", title="Model Info"))


def get_ollama_models() -> List[str]:
    """Get list of available Ollama models via HTTP API."""
    try:
        # Use the Ollama API endpoint
        with urllib.request.urlopen('http://localhost:11434/api/tags') as response:
            data = json.loads(response.read().decode())
            models = []
            for model in data.get('models', []):
                # Extract the model name (which includes tags like :8b, :latest)
                models.append(model['name'])
            return models
    except (urllib.error.URLError, json.JSONDecodeError, KeyError):
        return []


def get_supported_models() -> Dict[str, List[str]]:
    """Get all supported models grouped by provider."""
    models = {
        "OpenAI": [
            "gpt-3.5-turbo",
            "gpt-4",
            "gpt-4o",
            "gpt-4-turbo",
            "gpt-5",
            "chatgpt-4o-latest"
        ],
        "Gemini": [
            "gemini-2.5-flash",
            "gemini-2.5-pro",
            "gemini-1.5-flash",
            "gemini-1.5-pro"
        ],
        "Anthropic": [
            "claude-sonnet-4-20250514",
            "claude-opus-4-1-20250805",
            "sonnet",
            "opus",
            "claude-sonnet",
            "claude-opus"
        ],
        "Ollama": get_ollama_models()
    }
    return models


def print_models_list():
    """Print all supported models grouped by provider with API key status."""
    from rich.table import Table

    console.print("\n[bold]eunice - Available Models[/bold]")

    table = Table(show_header=True, header_style="bold magenta")
    table.add_column("Provider", style="cyan", no_wrap=True)
    table.add_column("Status", style="green")
    table.add_column("Models", style="white")

    models = get_supported_models()
    key_status = check_api_key_status()

    for provider, model_list in models.items():
        if provider == "OpenAI":
            icon = "🤖"
            key_name = "OPENAI_API_KEY"
        elif provider == "Gemini":
            icon = "💎"
            key_name = "GEMINI_API_KEY"
        elif provider == "Anthropic":
            icon = "🧠"
            key_name = "ANTHROPIC_API_KEY"
        else:  # Ollama
            icon = "🦙"
            key_name = None

        # Show API key status for providers that need it
        if key_name:
            if key_status.get(key_name):
                status = f"✅ API key set (...{key_status[key_name]})"
            else:
                status = f"❌ API key not set"
        else:
            status = "(local)"

        if not model_list:
            if provider == "Ollama":
                models_str = "[dim]No models installed. Use 'ollama pull <model>'[/dim]"
            else:
                models_str = "[dim]No models available[/dim]"
        else:
            models_str = "\n".join([f"• {model}" for model in model_list])

        table.add_row(f"{icon} {provider}", status, models_str)

    console.print(table)


def check_api_key_status() -> Dict[str, Optional[str]]:
    """Check the status of API keys and return last 4 characters if set."""
    keys = {}

    openai_key = os.getenv("OPENAI_API_KEY")
    if openai_key and len(openai_key) >= 4:
        keys["OPENAI_API_KEY"] = openai_key[-4:]
    else:
        keys["OPENAI_API_KEY"] = None

    gemini_key = os.getenv("GEMINI_API_KEY")
    if gemini_key and len(gemini_key) >= 4:
        keys["GEMINI_API_KEY"] = gemini_key[-4:]
    else:
        keys["GEMINI_API_KEY"] = None

    anthropic_key = os.getenv("ANTHROPIC_API_KEY")
    if anthropic_key and len(anthropic_key) >= 4:
        keys["ANTHROPIC_API_KEY"] = anthropic_key[-4:]
    else:
        keys["ANTHROPIC_API_KEY"] = None

    return keys


class CustomHelpFormatter(argparse.RawDescriptionHelpFormatter):
    """Custom help formatter."""

    def format_help(self):
        help_text = super().format_help()

        # Add a note about model listing
        extra_info = "\n" + "=" * 50 + "\n"
        extra_info += "Use --list-models to see all available models and API key status\n"

        return help_text + extra_info




class MCPServer:
    """Represents an MCP server with its process and tools."""
    def __init__(self, name: str, command: str, args: List[str]):
        self.name = name
        self.command = command
        self.args = args
        self.process = None
        self.tools = []

    async def start(self):
        """Start the MCP server process."""
        try:
            self.process = await asyncio.create_subprocess_exec(
                self.command, *self.args,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )

            # Give FastMCP time to initialize and output any startup messages
            await asyncio.sleep(2.0)

            # Initialize MCP protocol
            await self._send_message({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "clientInfo": {"name": "eunice", "version": "1.0.0"}
                }
            })

            init_response = await self._read_message()
            if init_response and init_response.get("result"):
                # Send initialized notification
                await self._send_notification({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                })

                # Discover tools
                await self._discover_tools()
                return True

        except Exception as e:
            print(f"Error starting MCP server {self.name}: {e}", file=sys.stderr)
            return False

    async def _send_message(self, message: Dict[str, Any]):
        """Send a JSON-RPC message to the server."""
        if self.process and self.process.stdin:
            message_str = json.dumps(message) + "\n"
            self.process.stdin.write(message_str.encode())
            await self.process.stdin.drain()

    async def _send_notification(self, notification: Dict[str, Any]):
        """Send a JSON-RPC notification to the server."""
        await self._send_message(notification)

    async def _read_message(self) -> Optional[Dict[str, Any]]:
        """Read a JSON-RPC message from the server."""
        if self.process and self.process.stdout:
            try:
                line = await asyncio.wait_for(self.process.stdout.readline(), timeout=600.0)
                if line:
                    line_str = line.decode().strip()
                    if line_str.startswith('{'):
                        return json.loads(line_str)
                    else:
                        return await self._read_message()  # Try reading next line
            except asyncio.TimeoutError:
                return None
            except json.JSONDecodeError as e:
                return None
            except Exception as e:
                return None
        return None

    async def _discover_tools(self):
        """Discover available tools from the server."""
        await self._send_message({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        })

        response = await self._read_message()
        if response and response.get("result") and "tools" in response["result"]:
            for tool in response["result"]["tools"]:
                # Prefix tool name with server name
                tool_name = f"{self.name}.{tool['name']}"
                tool_spec = {
                    "type": "function",
                    "function": {
                        "name": tool_name,
                        "description": tool.get("description", ""),
                        "parameters": tool.get("inputSchema", {"type": "object", "properties": {}})
                    }
                }
                self.tools.append(tool_spec)
        else:
            pass

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any], verbose: bool = False) -> str:
        """Call a tool on this server."""
        import time
        start_time = time.time()

        # Remove server prefix from tool name
        actual_tool_name = tool_name.split(".", 1)[1] if "." in tool_name else tool_name


        await self._send_message({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": actual_tool_name,
                "arguments": arguments
            }
        })


        response = await self._read_message()

        end_time = time.time()

        if response and response.get("result"):
            result = response["result"]
            if "content" in result and isinstance(result["content"], list):
                # MCP returns content as a list of content blocks
                content_parts = []
                for content_block in result["content"]:
                    if content_block.get("type") == "text":
                        content_parts.append(content_block.get("text", ""))
                return "\n".join(content_parts)
            return str(result)
        elif response and response.get("error"):
            return f"Error: {response['error'].get('message', 'Unknown error')}"
        else:
            return "Error: No response from MCP server"

    async def stop(self):
        """Stop the MCP server process."""
        if self.process:
            try:
                # First try graceful shutdown
                if self.process.stdin:
                    self.process.stdin.close()

                # Give it a moment to exit gracefully
                try:
                    await asyncio.wait_for(self.process.wait(), timeout=2.0)
                except asyncio.TimeoutError:
                    # Force terminate if it doesn't exit gracefully
                    self.process.terminate()
                    await self.process.wait()
            except Exception:
                # If anything goes wrong, just terminate
                try:
                    self.process.terminate()
                    await self.process.wait()
                except Exception:
                    pass  # Process might already be dead


class MCPManager:
    """Manages multiple MCP servers."""
    def __init__(self):
        self.servers = {}  # MCP servers
        self.tools = []

    async def load_config(self, config_path: str, silent: bool = False):
        """Load MCP server configuration from file."""
        if not silent:
            print(f"Loading configuration from: {config_path}", file=sys.stderr)
        try:
            with open(config_path, 'r') as f:
                config = json.load(f)

            # Load MCP servers
            if "mcpServers" in config:
                for server_name, server_config in config["mcpServers"].items():
                    if "command" not in server_config or "args" not in server_config:
                        if not silent:
                            print(f"Warning: Invalid configuration for MCP server {server_name}", file=sys.stderr)
                        continue

                    server = MCPServer(server_name, server_config["command"], server_config["args"])
                    if await server.start():
                        self.servers[server_name] = server
                        self.tools.extend(server.tools)
                        if not silent:
                            print(f"Started MCP server: {server_name}", file=sys.stderr)
                    else:
                        if not silent:
                            print(f"Failed to start MCP server: {server_name}", file=sys.stderr)

            if "mcpServers" not in config:
                if not silent:
                    print(f"Warning: No 'mcpServers' section found in {config_path}", file=sys.stderr)

        except Exception as e:
            if not silent:
                print(f"Error loading configuration: {e}", file=sys.stderr)
            raise

    def get_tools_spec(self) -> List[Dict[str, Any]]:
        """Return the OpenAI tools specification for all MCP tools."""
        return self.tools

    async def execute_tool(self, tool_name: str, arguments: Dict[str, Any], verbose: bool = False) -> str:
        """Execute a tool call by routing to the appropriate server."""
        start_time = time.time()

        if "." not in tool_name:
            return f"Error: Invalid tool name '{tool_name}' - tools must be prefixed with server name"

        server_name = tool_name.split(".", 1)[0]

        try:
            # Check MCP servers first
            if server_name in self.servers:
                result = await self.servers[server_name].call_tool(tool_name, arguments, verbose)
                end_time = time.time()
                return result


            else:
                return f"Error: Unknown server '{server_name}'"

        except Exception as e:
            end_time = time.time()
            return f"Error: {str(e)}"

    def print_server_info(self):
        """Print MCP server tool information in light yellow frames."""
        if not self.servers:
            return

        content_lines = ["🔌 MCP Servers & Tools", ""]

        for server_name, server in self.servers.items():
            content_lines.append(f"📡 {server_name}: {len(server.tools)} tools")
            for tool in server.tools:
                tool_name = tool["function"]["name"]
                content_lines.append(f"  • {tool_name}")

        content = "\n".join(content_lines)
        console.print(Panel(content, border_style="yellow", title="MCP Info"))

    async def shutdown(self):
        """Shutdown all MCP and streaming servers."""
        # Shut down all servers concurrently and suppress errors
        shutdown_tasks = []

        # Add MCP server shutdown tasks
        for server in self.servers.values():
            shutdown_tasks.append(server.stop())


        if shutdown_tasks:
            try:
                await asyncio.gather(*shutdown_tasks, return_exceptions=True)
            except Exception:
                pass  # Suppress any shutdown errors




def validate_ollama_model(model: str) -> bool:
    """Validate that an Ollama model is installed locally."""
    available_models = get_ollama_models()
    return model in available_models


def resolve_anthropic_model(model: str) -> str:
    """Resolve Anthropic model aliases to full model names."""
    model_lower = model.lower()

    # Alias mappings
    alias_map = {
        "sonnet": "claude-sonnet-4-20250514",
        "claude-sonnet": "claude-sonnet-4-20250514",
        "opus": "claude-opus-4-1-20250805",
        "claude-opus": "claude-opus-4-1-20250805"
    }

    return alias_map.get(model_lower, model)


def get_smart_default_model() -> str:
    """
    Get the best available model based on priority order:
    1. Ollama models: gpt-oss:latest, deepseek-r1:latest, llama3.1:latest
    2. Gemini: gemini-2.5-flash (if API key exists)
    3. Anthropic: sonnet (if API key exists)
    4. OpenAI: gpt-4o (if API key exists)
    """
    # Priority order for Ollama models
    preferred_ollama_models = ["gpt-oss:latest", "deepseek-r1:latest", "llama3.1:latest"]

    # Check Ollama models first
    available_ollama_models = get_ollama_models()
    for model in preferred_ollama_models:
        if model in available_ollama_models:
            return model

    # Check for API keys and return first available cloud model
    if os.getenv("GEMINI_API_KEY"):
        return "gemini-2.5-flash"

    if os.getenv("ANTHROPIC_API_KEY"):
        return "sonnet"

    if os.getenv("OPENAI_API_KEY"):
        return "gpt-4o"

    # Fallback to any available Ollama model if no API keys
    if available_ollama_models:
        return available_ollama_models[0]

    # Ultimate fallback
    return "gemini-2.5-flash"


def detect_provider(model: str) -> tuple[str, str, str]:
    """
    Detect the provider based on model name.
    Returns (provider, base_url, api_key)
    """
    model_lower = model.lower()

    # Gemini models (highest priority, explicit check)
    if model_lower.startswith("gemini"):
        api_key = os.getenv("GEMINI_API_KEY")
        if not api_key:
            raise ValueError(f"GEMINI_API_KEY environment variable is required for model '{model}'")
        return "gemini", "https://generativelanguage.googleapis.com/v1beta/openai/", api_key

    # Anthropic models (Claude) - check aliases and full names
    elif (model_lower.startswith("claude") or
          model_lower in ["sonnet", "opus", "claude-sonnet", "claude-opus"]):
        api_key = os.getenv("ANTHROPIC_API_KEY")
        if not api_key:
            raise ValueError(f"ANTHROPIC_API_KEY environment variable is required for model '{model}'")
        # Resolve model alias to full name
        resolved_model = resolve_anthropic_model(model)
        # Anthropic doesn't have OpenAI-compatible endpoints, but the openai library supports Anthropic
        return "anthropic", "https://api.anthropic.com/v1", api_key

    # Check if model is available in Ollama first (to handle cases like gpt-oss)
    elif validate_ollama_model(model):
        return "ollama", "http://localhost:11434/v1", "ollama"

    # OpenAI models (only if not found in Ollama)
    elif any(prefix in model_lower for prefix in ["gpt", "chatgpt", "davinci", "curie", "babbage", "ada"]):
        api_key = os.getenv("OPENAI_API_KEY")
        if not api_key:
            raise ValueError(f"OPENAI_API_KEY environment variable is required for model '{model}'")
        return "openai", "https://api.openai.com/v1", api_key

    # Fallback: try Ollama for any other model
    else:
        # Model not found in Ollama and doesn't match known patterns
        available_models = get_ollama_models()
        if available_models:
            models_list = ", ".join(available_models)
            raise ValueError(f"Model '{model}' not recognized. Available Ollama models: {models_list}. For other providers, try: gpt-* (OpenAI), gemini-* (Gemini). Try running: ollama pull {model}")
        else:
            raise ValueError(f"Model '{model}' not recognized and no Ollama models found. Try running: ollama pull {model}")

        return "ollama", "http://localhost:11434/v1", "ollama"


def create_client(model: str) -> OpenAI:
    """Create an OpenAI client based on the model."""
    provider, base_url, api_key = detect_provider(model)

    return OpenAI(
        api_key=api_key,
        base_url=base_url,
        timeout=600.0  # 10 minutes timeout
    )


async def run_agent(client: OpenAI, model: str, prompt: str, tool_output_limit: int = 50, mcp_manager: Optional[MCPManager] = None, silent: bool = False, verbose: bool = False, conversation_history: Optional[List[Dict[str, Any]]] = None, suppress_info: bool = False) -> List[Dict[str, Any]]:
    """Run the agent with the given prompt until completion."""

    if conversation_history is None:
        messages = [
            {"role": "user", "content": prompt}
        ]
    else:
        messages = conversation_history.copy()
        messages.append({"role": "user", "content": prompt})

    # Show MCP server info if available (unless suppressed for interactive mode)
    if mcp_manager and not silent and not suppress_info:
        mcp_manager.print_server_info()

    # Show model info (unless suppressed for interactive mode)
    if not suppress_info:
        provider, _, _ = detect_provider(model)
        print_model_info(model, provider, silent)

    tools = mcp_manager.get_tools_spec() if mcp_manager else []


    # Resolve model name for API call (handles Anthropic aliases)
    resolved_model = resolve_anthropic_model(model)

    while True:
        try:
            response = client.chat.completions.create(
                model=resolved_model,
                messages=messages,
                tools=tools if tools else None,
                tool_choice="auto" if tools else None
            )

            message = response.choices[0].message
            messages.append({
                "role": "assistant",
                "content": message.content,
                "tool_calls": message.tool_calls if hasattr(message, 'tool_calls') and message.tool_calls else None
            })

            # Print assistant's response if there is content
            if message.content:
                print(message.content)

            # If there are no tool calls, we're done
            if not hasattr(message, 'tool_calls') or not message.tool_calls:
                break

            # Execute tool calls
            for tool_call in message.tool_calls:
                tool_name = tool_call.function.name
                tool_args = json.loads(tool_call.function.arguments)

                # Print formatted tool invocation
                print_tool_invocation(tool_name, tool_args, silent)

                # Execute the tool via MCP manager

                if mcp_manager:
                    import time
                    start_time = time.time()
                    tool_result = await mcp_manager.execute_tool(tool_name, tool_args, verbose)
                    end_time = time.time()
                else:
                    tool_result = f"Error: No tools available (no MCP configuration)"

                # Print formatted tool result
                print_tool_result(tool_result, tool_output_limit, silent)

                # Add tool result to messages
                messages.append({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": tool_result
                })

        except Exception as e:
            print(f"Error: {str(e)}", file=sys.stderr)
            sys.exit(1)

    return messages


async def simple_interactive_mode(client: OpenAI, model: str, initial_prompt: Optional[str], tool_output_limit: int = 50, mcp_manager: Optional[MCPManager] = None, silent: bool = False, verbose: bool = False) -> None:
    """Minimal interactive implementation."""
    conversation_history = []

    # Show MCP server info and model info once at startup
    if mcp_manager and not silent:
        mcp_manager.print_server_info()

    # Show model info once at startup
    provider, _, _ = detect_provider(model)
    print_model_info(model, provider, silent)

    # Process initial prompt if provided
    if initial_prompt:
        conversation_history = await run_agent(client, model, initial_prompt, tool_output_limit, mcp_manager, silent, verbose, conversation_history, suppress_info=True)

    print("\n🔄 Interactive mode. Type 'exit' or 'quit' to end session.")

    while True:
        try:
            user_input = input("\n> ").strip()
            if user_input.lower() in ['exit', 'quit']:
                break
            if not user_input:
                continue

            # Process with existing logic but pass conversation_history and suppress info display
            conversation_history = await run_agent(client, model, user_input, tool_output_limit, mcp_manager, silent, verbose, conversation_history, suppress_info=True)

        except KeyboardInterrupt:
            print("\n👋 Session ended.")
            break
        except EOFError:
            break


def parse_prompt_arg(prompt_arg: str) -> str:
    """Parse the prompt argument - could be a file path or a string."""
    # Quick checks to avoid filesystem operations on obviously non-file strings
    if (len(prompt_arg) > 255 or  # Most filesystems have 255 char limit
        '\n' in prompt_arg or     # File paths shouldn't have newlines
        '?' in prompt_arg or      # Question marks are unlikely in file paths
        prompt_arg.count(' ') > 5):  # File paths with many spaces are unlikely
        # This is definitely a string prompt, not a file path
        return prompt_arg

    # Only check filesystem if it could plausibly be a file path
    try:
        path = Path(prompt_arg)
        if path.exists() and path.is_file():
            try:
                with open(path, 'r', encoding='utf-8') as f:
                    return f.read()
            except Exception as e:
                print(f"Error reading prompt file {prompt_arg}: {str(e)}", file=sys.stderr)
                sys.exit(1)
        elif '.' in prompt_arg and ('/' in prompt_arg or '\\' in prompt_arg or prompt_arg.endswith('.txt') or prompt_arg.endswith('.md')):
            # This looks like a file path but the file doesn't exist
            print(f"Error reading prompt file {prompt_arg}: File does not exist", file=sys.stderr)
            sys.exit(1)
    except (OSError, ValueError):
        # If we get filesystem errors, just treat it as a string prompt
        pass

    # It's a string prompt
    return prompt_arg


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="eunice - Agentic CLI runner",
        formatter_class=CustomHelpFormatter
    )

    parser.add_argument(
        "--version",
        action="version",
        version=f"eunice {__version__}"
    )

    parser.add_argument(
        "--model",
        default=None,
        help="Model to use (default: smart selection based on availability)"
    )

    parser.add_argument(
        "--prompt",
        help="Prompt to send to the model (can be a file path or string)"
    )

    parser.add_argument(
        "--tool-output-limit",
        type=int,
        default=50,
        help="Limit tool output to N characters (default: 50, use 0 for no limit)"
    )

    parser.add_argument(
        "--list-models",
        action="store_true",
        help="List all supported models grouped by provider"
    )

    parser.add_argument(
        "--config",
        help="Path to MCP server configuration JSON file"
    )

    parser.add_argument(
        "--silent",
        action="store_true",
        help="Suppress all output except AI responses (hide tool calls and model info)"
    )

    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable verbose debug output to /tmp/eunice_debug.log"
    )

    parser.add_argument(
        "--no-mcp",
        action="store_true",
        help="Disable MCP server loading even if eunice.json exists"
    )

    parser.add_argument(
        "--interact",
        action="store_true",
        help="Enable interactive mode for multi-turn conversations"
    )

    parser.add_argument(
        "prompt_positional",
        nargs="?",
        help="Prompt as positional argument (can be a file path or string)"
    )

    args = parser.parse_args()

    # Handle --list-models option
    if args.list_models:
        print_models_list()
        sys.exit(0)

    # Validate --no-mcp and --config arguments
    if args.no_mcp and args.config is not None:
        print("Error: --no-mcp and --config cannot be used together", file=sys.stderr)
        sys.exit(1)

    # Determine the prompt and interactive mode
    prompt = None
    interactive_mode = args.interact

    if args.prompt is not None:
        prompt = parse_prompt_arg(args.prompt)
    elif args.prompt_positional is not None:
        prompt = parse_prompt_arg(args.prompt_positional)
    else:
        # No prompt provided - default to interactive mode
        interactive_mode = True

    # Determine the model to use (smart default if not specified)
    model = args.model if args.model is not None else get_smart_default_model()

    # Create client and run agent
    try:
        client = create_client(model)

        # Determine config path: explicit --config takes precedence, then check for eunice.json in current working directory
        config_path = None

        if not args.no_mcp:
            if args.config is not None:
                # Handle empty config parameter (--config='' should function like --no-mcp)
                if args.config.strip() == '':
                    config_path = None
                else:
                    config_path = args.config
            elif Path.cwd().joinpath("eunice.json").exists():
                config_path = str(Path.cwd().joinpath("eunice.json"))

        # Run everything in a single asyncio context
        asyncio.run(main_async(client, model, prompt, args.tool_output_limit, config_path, args.silent, args.verbose, interactive_mode))

    except Exception as e:
        print(f"Error: {str(e)}", file=sys.stderr)
        sys.exit(1)


async def main_async(client: OpenAI, model: str, prompt: Optional[str], tool_output_limit: int, config_path: Optional[str], silent: bool = False, verbose: bool = False, interactive: bool = False):
    """Main async function that handles MCP setup and agent execution."""
    mcp_manager = None

    try:
        # Create and configure MCP manager if config provided
        if config_path:
            mcp_manager = MCPManager()
            await mcp_manager.load_config(config_path, silent)

        # Run the appropriate mode
        if interactive:
            await simple_interactive_mode(client, model, prompt, tool_output_limit, mcp_manager, silent, verbose)
        else:
            await run_agent(client, model, prompt, tool_output_limit, mcp_manager, silent, verbose)

    finally:
        # Always cleanup MCP servers if they exist
        if mcp_manager:
            await mcp_manager.shutdown()


if __name__ == "__main__":
    main()