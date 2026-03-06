"""Conversation agent — runs a tool-using dialogue loop via Anthropic SDK."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import anthropic
from rich.console import Console
from rich.markdown import Markdown
from rich.panel import Panel
from rich.prompt import Prompt


@dataclass
class AgentConfig:
    """Configuration for an agent."""

    name: str
    system_prompt: str
    model: str = "claude-sonnet-4-20250514"
    color: str = "green"
    tools: list[str] = field(default_factory=list)
    max_tokens: int = 4096


# Tool definitions for the Anthropic API.
TOOL_DEFINITIONS: dict[str, dict[str, Any]] = {
    "read_file": {
        "name": "read_file",
        "description": "Read the contents of a file at the given path.",
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative file path to read.",
                },
            },
            "required": ["path"],
        },
    },
    "write_file": {
        "name": "write_file",
        "description": "Write content to a file at the given path. Creates parent directories if needed.",
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to write to.",
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file.",
                },
            },
            "required": ["path", "content"],
        },
    },
    "list_files": {
        "name": "list_files",
        "description": "List files and directories at the given path.",
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list.",
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth to recurse. Default 2.",
                    "default": 2,
                },
            },
            "required": ["path"],
        },
    },
}


class Agent:
    """A tool-using conversation agent backed by Claude."""

    def __init__(self, config: AgentConfig, project_path: Path) -> None:
        self.config = config
        self.project_path = project_path.resolve()
        self.client = anthropic.Anthropic()
        self.console = Console()
        self.messages: list[dict[str, Any]] = []

    def _resolve_path(self, path: str) -> Path:
        """Resolve a path relative to the project root."""
        p = Path(path)
        if p.is_absolute():
            return p
        return self.project_path / p

    def _execute_tool(self, name: str, input_data: dict[str, Any]) -> str:
        """Execute a tool and return the result as a string."""
        if name == "read_file":
            path = self._resolve_path(input_data["path"])
            if not path.exists():
                return f"Error: {path} does not exist."
            if not path.is_file():
                return f"Error: {path} is not a file."
            return path.read_text()

        if name == "write_file":
            path = self._resolve_path(input_data["path"])
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(input_data["content"])
            return f"Wrote {len(input_data['content'])} chars to {path}"

        if name == "list_files":
            path = self._resolve_path(input_data["path"])
            max_depth = input_data.get("max_depth", 2)
            if not path.exists():
                return f"Error: {path} does not exist."
            return _tree(path, max_depth)

        return f"Error: unknown tool {name}"

    def _get_tools(self) -> list[dict[str, Any]]:
        """Return tool definitions for the configured tools."""
        return [TOOL_DEFINITIONS[t] for t in self.config.tools if t in TOOL_DEFINITIONS]

    def _display_response(self, text: str) -> None:
        """Display agent response in a styled panel."""
        self.console.print(
            Panel(
                Markdown(text),
                title=f"[bold {self.config.color}]{self.config.name}[/]",
                border_style=self.config.color,
                padding=(1, 2),
            )
        )

    def run(self) -> None:
        """Run the conversation loop."""
        self.console.print(
            f"\n[bold {self.config.color}]{self.config.name}[/] is ready. "
            f"Project: [dim]{self.project_path}[/]\n"
            "[dim]Type 'exit' or 'quit' to end the session.[/]\n"
        )

        while True:
            user_input = Prompt.ask(f"[bold]you[/]")

            if user_input.strip().lower() in ("exit", "quit", "q"):
                self.console.print("[dim]Session ended.[/]")
                break

            if not user_input.strip():
                continue

            self.messages.append({"role": "user", "content": user_input})
            self._turn()

    def _turn(self) -> None:
        """Execute one agent turn (may involve multiple tool-use rounds)."""
        tools = self._get_tools()

        while True:
            response = self.client.messages.create(
                model=self.config.model,
                max_tokens=self.config.max_tokens,
                system=self.config.system_prompt,
                messages=self.messages,
                tools=tools if tools else anthropic.NOT_GIVEN,
            )

            # Collect text blocks and tool_use blocks.
            text_parts: list[str] = []
            tool_calls: list[dict[str, Any]] = []

            for block in response.content:
                if block.type == "text":
                    text_parts.append(block.text)
                elif block.type == "tool_use":
                    tool_calls.append(
                        {"id": block.id, "name": block.name, "input": block.input}
                    )

            # Display any text.
            if text_parts:
                self._display_response("\n".join(text_parts))

            # Append the full assistant message.
            self.messages.append({"role": "assistant", "content": response.content})

            # If no tool calls or stop reason isn't tool_use, we're done.
            if response.stop_reason != "tool_use" or not tool_calls:
                break

            # Execute tools and send results back.
            tool_results = []
            for call in tool_calls:
                self.console.print(
                    f"  [dim]tool:[/] {call['name']}({_summarize_input(call['input'])})"
                )
                result = self._execute_tool(call["name"], call["input"])
                tool_results.append(
                    {
                        "type": "tool_result",
                        "tool_use_id": call["id"],
                        "content": result,
                    }
                )

            self.messages.append({"role": "user", "content": tool_results})


def _summarize_input(input_data: dict[str, Any]) -> str:
    """Summarize tool input for display."""
    parts = []
    for k, v in input_data.items():
        if isinstance(v, str) and len(v) > 60:
            parts.append(f"{k}=...{len(v)} chars")
        else:
            parts.append(f"{k}={json.dumps(v)}")
    return ", ".join(parts)


def _tree(path: Path, max_depth: int, prefix: str = "", depth: int = 0) -> str:
    """Generate a tree-like listing of a directory."""
    if depth > max_depth:
        return ""

    lines: list[str] = []
    if depth == 0:
        lines.append(str(path))

    try:
        entries = sorted(path.iterdir(), key=lambda p: (not p.is_dir(), p.name))
    except PermissionError:
        return prefix + "[permission denied]"

    # Skip hidden dirs and common noise.
    entries = [
        e
        for e in entries
        if not e.name.startswith(".")
        and e.name not in ("__pycache__", "node_modules", ".venv", "target")
    ]

    for i, entry in enumerate(entries):
        is_last = i == len(entries) - 1
        connector = "└── " if is_last else "├── "
        lines.append(f"{prefix}{connector}{entry.name}")

        if entry.is_dir() and depth < max_depth:
            extension = "    " if is_last else "│   "
            subtree = _tree(entry, max_depth, prefix + extension, depth + 1)
            if subtree:
                lines.append(subtree)

    return "\n".join(lines)
