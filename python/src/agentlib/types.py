"""Core types for AgentLib."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Optional


class Role(str, Enum):
    USER = "user"
    ASSISTANT = "assistant"


@dataclass(frozen=True)
class TextBlock:
    text: str
    type: str = field(default="text", init=False)


@dataclass(frozen=True)
class ToolUseBlock:
    name: str
    id: str
    input: dict
    type: str = field(default="tool_use", init=False)


@dataclass(frozen=True)
class ToolResultBlock:
    tool_use_id: str
    content: str
    is_error: Optional[bool] = None
    type: str = field(default="tool_result", init=False)


@dataclass(frozen=True)
class ThinkingBlock:
    thinking: str
    signature: Optional[str] = None
    type: str = field(default="thinking", init=False)


ContentBlock = TextBlock | ToolUseBlock | ToolResultBlock | ThinkingBlock


@dataclass
class Message:
    role: Role
    content: list[ContentBlock]

    @staticmethod
    def user(text: str) -> Message:
        return Message(role=Role.USER, content=[TextBlock(text=text)])

    @staticmethod
    def assistant(blocks: list[ContentBlock]) -> Message:
        return Message(role=Role.ASSISTANT, content=blocks)


class EventType(str, Enum):
    TURN_START = "turn_start"
    MESSAGE_START = "message_start"
    MESSAGE_DELTA = "message_delta"
    THINKING_DELTA = "thinking_delta"
    MESSAGE_END = "message_end"
    TOOL_USE_START = "tool_use_start"
    TOOL_USE_END = "tool_use_end"
    ERROR = "error"
    COMPLETE = "complete"
    COMPACT = "compact"


@dataclass
class Event:
    type: EventType
    data: dict

    @staticmethod
    def turn_start(turn: int) -> Event:
        return Event(type=EventType.TURN_START, data={"turn": turn})

    @staticmethod
    def message_start() -> Event:
        return Event(type=EventType.MESSAGE_START, data={})

    @staticmethod
    def message_delta(text: str) -> Event:
        return Event(type=EventType.MESSAGE_DELTA, data={"text": text})

    @staticmethod
    def thinking_delta(thinking: str) -> Event:
        return Event(type=EventType.THINKING_DELTA, data={"thinking": thinking})

    @staticmethod
    def message_end() -> Event:
        return Event(type=EventType.MESSAGE_END, data={})

    @staticmethod
    def tool_use_start(name: str, id: str, input: dict) -> Event:
        return Event(type=EventType.TOOL_USE_START, data={"name": name, "id": id, "input": input})

    @staticmethod
    def tool_use_end(name: str, id: str, result: "ToolResult") -> Event:
        return Event(type=EventType.TOOL_USE_END, data={"name": name, "id": id, "result": result})

    @staticmethod
    def error(message: str) -> Event:
        return Event(type=EventType.ERROR, data={"message": message})

    @staticmethod
    def complete(final_content: str) -> Event:
        return Event(type=EventType.COMPLETE, data={"final_content": final_content})

    @staticmethod
    def compact(message_count: int, token_estimate: int) -> Event:
        return Event(
            type=EventType.COMPACT,
            data={"message_count": message_count, "token_estimate": token_estimate},
        )
