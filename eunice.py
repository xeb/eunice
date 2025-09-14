#!/usr/bin/env python3
# /// script
# requires-python = ">=3.13"
# dependencies = [
#     "openai",
# ]
# ///
"""
eunice - Agentic CLI runner
Usage: eunice [--model=MODEL] [--prompt=PROMPT] [prompt]
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

import openai
from openai import OpenAI

# Color constants for terminal output
class Colors:
    RESET = '\033[0m'
    BOLD = '\033[1m'
    DIM = '\033[2m'

    # Regular colors
    RED = '\033[31m'
    GREEN = '\033[32m'
    YELLOW = '\033[33m'
    BLUE = '\033[34m'
    MAGENTA = '\033[35m'
    CYAN = '\033[36m'
    WHITE = '\033[37m'

    # Light colors
    LIGHT_RED = '\033[91m'
    LIGHT_GREEN = '\033[92m'
    LIGHT_YELLOW = '\033[93m'
    LIGHT_BLUE = '\033[94m'
    LIGHT_MAGENTA = '\033[95m'
    LIGHT_CYAN = '\033[96m'
    LIGHT_WHITE = '\033[97m'

def print_tool_invocation(tool_name: str, tool_args: Dict[str, Any]) -> None:
    """Print a formatted tool invocation with light blue color and framing."""
    args_str = json.dumps(tool_args, indent=None, separators=(',', ':'))

    # Create the frame
    content = f"🔧 {tool_name}({args_str})"
    frame_width = max(50, len(content) + 4)

    print(f"\n{Colors.LIGHT_BLUE}┌{'─' * (frame_width - 2)}┐{Colors.RESET}")
    print(f"{Colors.LIGHT_BLUE}│ {Colors.BOLD}{content:<{frame_width - 4}} {Colors.RESET}{Colors.LIGHT_BLUE}│{Colors.RESET}")
    print(f"{Colors.LIGHT_BLUE}└{'─' * (frame_width - 2)}┘{Colors.RESET}")

def print_tool_result(result: str, output_limit: int = 50) -> None:
    """Print a formatted tool result with green color and framing."""
    original_length = len(result)
    truncated_chars = 0

    # Apply truncation if limit > 0
    if output_limit > 0 and len(result) > output_limit:
        result = result[:output_limit]
        truncated_chars = original_length - output_limit

    lines = result.strip().split('\n')
    max_width = max(50, max(len(line) for line in lines) + 4)

    # If we have truncated characters, add a truncation notice
    if truncated_chars > 0:
        truncation_notice = f"...{truncated_chars} characters truncated"
        lines.append(truncation_notice)
        max_width = max(max_width, len(truncation_notice) + 4)

    print(f"{Colors.GREEN}┌{'─' * (max_width - 2)}┐{Colors.RESET}")
    print(f"{Colors.GREEN}│ {Colors.BOLD}Result:{' ' * (max_width - 9)}{Colors.RESET}{Colors.GREEN}│{Colors.RESET}")
    print(f"{Colors.GREEN}├{'─' * (max_width - 2)}┤{Colors.RESET}")

    for i, line in enumerate(lines):
        padded_line = f"{line:<{max_width - 4}}"
        # If this is the truncation notice, make it dim
        if truncated_chars > 0 and i == len(lines) - 1:
            print(f"{Colors.GREEN}│ {Colors.DIM}{Colors.LIGHT_GREEN}{padded_line}{Colors.RESET}{Colors.GREEN} │{Colors.RESET}")
        else:
            print(f"{Colors.GREEN}│ {Colors.LIGHT_GREEN}{padded_line}{Colors.RESET}{Colors.GREEN} │{Colors.RESET}")

    print(f"{Colors.GREEN}└{'─' * (max_width - 2)}┘{Colors.RESET}")
    print()  # Add spacing after result


def get_ollama_models() -> List[str]:
    """Get list of available Ollama models by running 'ollama list'."""
    try:
        result = subprocess.run(['ollama', 'list'], capture_output=True, text=True, check=True)
        lines = result.stdout.strip().split('\n')

        # Skip header line and extract model names
        models = []
        for line in lines[1:]:  # Skip the header
            if line.strip():
                # Model name is the first column, split by whitespace
                model_name = line.split()[0]
                # Remove the tag part (everything after :)
                if ':' in model_name:
                    model_name = model_name.split(':')[0]
                models.append(model_name)

        return models
    except (subprocess.CalledProcessError, FileNotFoundError):
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
        "Ollama": get_ollama_models()
    }
    return models


def print_models_list():
    """Print all supported models grouped by provider."""
    print(f"{Colors.BOLD}eunice - Available Models{Colors.RESET}")
    print("=" * 30)

    models = get_supported_models()

    for provider, model_list in models.items():
        if provider == "OpenAI":
            icon = "🤖"
            color = Colors.GREEN
        elif provider == "Gemini":
            icon = "💎"
            color = Colors.BLUE
        else:  # Ollama
            icon = "🦙"
            color = Colors.MAGENTA

        print(f"\n{color}{Colors.BOLD}{icon} {provider} Models:{Colors.RESET}")

        if not model_list:
            if provider == "Ollama":
                print(f"  {Colors.DIM}No Ollama models installed. Use 'ollama pull <model>' to install.{Colors.RESET}")
            else:
                print(f"  {Colors.DIM}No models available{Colors.RESET}")
        else:
            for model in model_list:
                print(f"  • {model}")


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

    return keys


class CustomHelpFormatter(argparse.RawDescriptionHelpFormatter):
    """Custom help formatter that adds models and API key status."""

    def format_help(self):
        help_text = super().format_help()

        # Add models and API key status
        extra_info = "\n" + "=" * 50 + "\n"
        extra_info += f"{Colors.BOLD}Available Models:{Colors.RESET}\n"

        models = get_supported_models()
        for provider, model_list in models.items():
            if provider == "OpenAI":
                icon = "🤖"
                color = Colors.GREEN
            elif provider == "Gemini":
                icon = "💎"
                color = Colors.BLUE
            else:  # Ollama
                icon = "🦙"
                color = Colors.MAGENTA

            extra_info += f"\n{color}{Colors.BOLD}{icon} {provider}:{Colors.RESET}\n"

            if not model_list:
                if provider == "Ollama":
                    extra_info += f"  {Colors.DIM}No models installed{Colors.RESET}\n"
                else:
                    extra_info += f"  {Colors.DIM}No models available{Colors.RESET}\n"
            else:
                for model in model_list[:3]:  # Show first 3 models
                    extra_info += f"  • {model}\n"
                if len(model_list) > 3:
                    extra_info += f"  • ... and {len(model_list) - 3} more\n"

        # Add API key status
        extra_info += f"\n{Colors.BOLD}API Key Status:{Colors.RESET}\n"
        key_status = check_api_key_status()

        for key_name, last_chars in key_status.items():
            if last_chars:
                extra_info += f"  ✅ {key_name} set (...{last_chars})\n"
            else:
                extra_info += f"  ❌ {key_name} not set\n"

        extra_info += f"\n{Colors.DIM}Use --list-models to see all available models{Colors.RESET}\n"

        return help_text + extra_info


def list_files(path: str) -> List[Dict[str, str]]:
    """List files in the given directory path."""
    try:
        path_obj = Path(path)
        if not path_obj.exists():
            return [{"error": f"Path {path} does not exist"}]
        if not path_obj.is_dir():
            return [{"error": f"Path {path} is not a directory"}]

        files = []
        for item in path_obj.iterdir():
            if item.is_file():
                files.append({"name": item.name, "type": "file"})
            elif item.is_dir():
                files.append({"name": item.name, "type": "directory"})
        return files
    except Exception as e:
        return [{"error": str(e)}]


def read_file(path: str) -> str:
    """Read and return the contents of a file."""
    try:
        path_obj = Path(path)
        if not path_obj.exists():
            return f"Error: File {path} does not exist"
        if not path_obj.is_file():
            return f"Error: {path} is not a file"

        with open(path_obj, 'r', encoding='utf-8') as f:
            return f.read()
    except Exception as e:
        return f"Error reading file: {str(e)}"


def get_tools_spec():
    """Return the OpenAI tools specification for our supported tools."""
    return [
        {
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files and directories in a given path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The directory path to list files from"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }
    ]


def execute_tool(tool_name: str, arguments: Dict[str, Any]) -> str:
    """Execute a tool function and return the result."""
    if tool_name == "list_files":
        path = arguments.get("path", ".")
        result = list_files(path)
        return json.dumps(result, indent=2)
    elif tool_name == "read_file":
        path = arguments.get("path", "")
        result = read_file(path)
        return result
    else:
        return f"Error: Unknown tool '{tool_name}'"


def validate_ollama_model(model: str) -> bool:
    """Validate that an Ollama model is installed locally."""
    available_models = get_ollama_models()
    return model in available_models


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
        base_url=base_url
    )


def run_agent(client: OpenAI, model: str, prompt: str, tool_output_limit: int = 50) -> None:
    """Run the agent with the given prompt until completion."""
    messages = [
        {"role": "user", "content": prompt}
    ]

    tools = get_tools_spec()

    while True:
        try:
            response = client.chat.completions.create(
                model=model,
                messages=messages,
                tools=tools,
                tool_choice="auto"
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
                print_tool_invocation(tool_name, tool_args)

                # Execute the tool
                tool_result = execute_tool(tool_name, tool_args)

                # Print formatted tool result
                print_tool_result(tool_result, tool_output_limit)

                # Add tool result to messages
                messages.append({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": tool_result
                })

        except Exception as e:
            print(f"Error: {str(e)}", file=sys.stderr)
            sys.exit(1)


def parse_prompt_arg(prompt_arg: str) -> str:
    """Parse the prompt argument - could be a file path or a string."""
    # Check if it's a file path
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

    # It's a string prompt
    return prompt_arg


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="eunice - Agentic CLI runner",
        formatter_class=CustomHelpFormatter
    )

    parser.add_argument(
        "--model",
        default="gpt-3.5-turbo",
        help="Model to use (default: gpt-3.5-turbo)"
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
        "prompt_positional",
        nargs="?",
        help="Prompt as positional argument (can be a file path or string)"
    )

    args = parser.parse_args()

    # Handle --list-models option
    if args.list_models:
        print_models_list()
        sys.exit(0)

    # Determine the prompt
    if args.prompt is not None:
        prompt = parse_prompt_arg(args.prompt)
    elif args.prompt_positional is not None:
        prompt = parse_prompt_arg(args.prompt_positional)
    else:
        print("Error: No prompt provided", file=sys.stderr)
        sys.exit(1)

    # Create client and run agent
    try:
        client = create_client(args.model)
        run_agent(client, args.model, prompt, args.tool_output_limit)
    except Exception as e:
        print(f"Error: {str(e)}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()