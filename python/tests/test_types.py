"""Tests for core types."""

import pytest

from agentlib import (
    Event,
    EventType,
    Message,
    Role,
    TextBlock,
    ThinkingBlock,
    ToolResultBlock,
    ToolUseBlock,
)


class TestMessage:
    def test_user_message(self):
        msg = Message.user("hello")
        assert msg.role == Role.USER
        assert len(msg.content) == 1
        assert isinstance(msg.content[0], TextBlock)
        assert msg.content[0].text == "hello"

    def test_assistant_message(self):
        blocks = [TextBlock(text="hi")]
        msg = Message.assistant(blocks)
        assert msg.role == Role.ASSISTANT
        assert len(msg.content) == 1


class TestContentBlocks:
    def test_text_block(self):
        block = TextBlock(text="hello")
        assert block.type == "text"
        assert block.text == "hello"

    def test_tool_use_block(self):
        block = ToolUseBlock(
            name="read_file", id="tu_01", input={"file_path": "/tmp/test.txt"}
        )
        assert block.type == "tool_use"
        assert block.name == "read_file"
        assert block.input["file_path"] == "/tmp/test.txt"

    def test_tool_result_block(self):
        block = ToolResultBlock(
            tool_use_id="tu_01", content="success", is_error=False
        )
        assert block.type == "tool_result"
        assert block.tool_use_id == "tu_01"
        assert block.is_error is False

    def test_thinking_block(self):
        block = ThinkingBlock(thinking="Let me think...")
        assert block.type == "thinking"
        assert block.thinking == "Let me think..."


class TestEvent:
    def test_turn_start(self):
        e = Event.turn_start(1)
        assert e.type == EventType.TURN_START
        assert e.data["turn"] == 1

    def test_message_delta(self):
        e = Event.message_delta("hello")
        assert e.type == EventType.MESSAGE_DELTA
        assert e.data["text"] == "hello"

    def test_thinking_delta(self):
        e = Event.thinking_delta("thinking...")
        assert e.type == EventType.THINKING_DELTA
        assert e.data["thinking"] == "thinking..."

    def test_error(self):
        e = Event.error("something went wrong")
        assert e.type == EventType.ERROR
        assert e.data["message"] == "something went wrong"

    def test_complete(self):
        e = Event.complete("done")
        assert e.type == EventType.COMPLETE
        assert e.data["final_content"] == "done"
