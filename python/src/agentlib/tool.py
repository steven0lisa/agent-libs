"""Tool protocol and related types."""

from __future__ import annotations

from typing import Protocol, runtime_checkable
from dataclasses import dataclass

from .types import Message


@dataclass
class ToolContext:
    work_dir: str
    message_history: list[Message]


@dataclass
class ToolResult:
    content: str
    is_error: bool = False

    @staticmethod
    def success(content: str) -> ToolResult:
        return ToolResult(content=content, is_error=False)

    @staticmethod
    def error(content: str) -> ToolResult:
        return ToolResult(content=content, is_error=True)


@runtime_checkable
class Tool(Protocol):
    """Protocol for tools that can be registered with an Agent."""

    @property
    def name(self) -> str: ...

    @property
    def description(self) -> str: ...

    @property
    def input_schema(self) -> dict: ...

    @property
    def is_read_only(self) -> bool:
        return False

    async def call(self, input: dict, context: ToolContext) -> ToolResult: ...
