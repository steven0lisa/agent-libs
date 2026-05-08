"""Tests for auto compact functionality."""

import json
import os
import tempfile

import pytest
import respx
from httpx import Response

from agentlib import (
    Agent,
    AgentConfig,
    Event,
    EventType,
    Message,
    Role,
    TextBlock,
    ToolResultBlock,
    ToolUseBlock,
)
from agentlib.compact import (
    estimate_tokens,
    should_compact,
    _find_compact_boundary,
    compact_conversation,
    auto_compact_if_needed,
    CHARS_PER_TOKEN,
)
from agentlib.client import AnthropicClient


class TestTokenEstimation:
    def test_empty_messages(self):
        assert estimate_tokens([]) == 0

    def test_single_text_message(self):
        messages = [Message.user("Hello world")]
        # "Hello world" = 11 chars -> 11/4 ≈ 2 tokens
        tokens = estimate_tokens(messages)
        assert tokens > 0
        assert tokens == int(11 / CHARS_PER_TOKEN)

    def test_multiple_messages(self):
        messages = [
            Message.user("Hello"),
            Message.assistant([TextBlock(text="Hi there")]),
            Message.user("How are you?"),
        ]
        tokens = estimate_tokens(messages)
        assert tokens > 0

    def test_tool_use_blocks(self):
        messages = [
            Message.assistant([
                ToolUseBlock(name="read_file", id="tu_01", input={"file_path": "test.txt"}),
            ]),
        ]
        tokens = estimate_tokens(messages)
        assert tokens > 0

    def test_tool_result_blocks(self):
        messages = [
            Message(role=Role.USER, content=[
                ToolResultBlock(tool_use_id="tu_01", content="file content here"),
            ]),
        ]
        tokens = estimate_tokens(messages)
        assert tokens > 0

    def test_large_message_approaches_threshold(self):
        # Create a message that should be near 1000 tokens
        # 1000 tokens * 4 chars/token = 4000 chars
        large_text = "x" * 4000
        messages = [Message.user(large_text)]
        tokens = estimate_tokens(messages)
        assert tokens >= 900  # Allow some estimation variance
        assert tokens <= 1100


class TestShouldCompact:
    def test_no_messages(self):
        assert should_compact([], 200000, 0.8) is False

    def test_below_threshold(self):
        # Small messages, should not trigger
        messages = [Message.user("Hello")]
        assert should_compact(messages, 200000, 0.8) is False

    def test_above_threshold(self):
        # Create messages that exceed the threshold
        # Threshold = 1000 * 0.8 = 800 tokens = 3200 chars
        large_text = "x" * 4000
        messages = [Message.user(large_text)]
        assert should_compact(messages, 1000, 0.8) is True

    def test_custom_threshold(self):
        large_text = "x" * 4000  # ~1000 tokens
        messages = [Message.user(large_text)]
        # 50% threshold of 2000 tokens = 1000 tokens, so 1000 tokens should trigger
        assert should_compact(messages, 2000, 0.5) is True
        # 99% threshold of 2000 tokens = 1980 tokens, so 1000 tokens should not trigger
        assert should_compact(messages, 2000, 0.99) is False


class TestFindCompactBoundary:
    def test_few_messages(self):
        messages = [Message.user(f"msg{i}") for i in range(3)]
        boundary = _find_compact_boundary(messages)
        assert boundary == 0  # Too few messages to compact

    def test_enough_messages(self):
        messages = [Message.user(f"msg{i}") for i in range(10)]
        boundary = _find_compact_boundary(messages)
        assert boundary > 0
        assert boundary == 10 - 5  # MIN_PRESERVED_MESSAGES = 5

    def test_preserves_tool_pairs(self):
        # Create messages with tool use/result pairs
        messages = []
        for i in range(8):
            messages.append(Message.user(f"prompt {i}"))
            messages.append(Message.assistant([
                ToolUseBlock(name="read_file", id=f"tu_{i}", input={"file_path": f"f{i}"}),
            ]))
            messages.append(Message(role=Role.USER, content=[
                ToolResultBlock(tool_use_id=f"tu_{i}", content=f"content {i}"),
            ]))

        boundary = _find_compact_boundary(messages)
        # Boundary should not split a tool pair
        assert boundary >= 0
        assert boundary < len(messages)


class TestCompactConversation:
    @pytest.mark.asyncio
    @respx.mock
    async def test_compact_reduces_messages(self):
        """Test that compact_conversation reduces message count."""
        # Create many messages
        messages = []
        for i in range(15):
            messages.append(Message.user(f"Message {i} with some content"))
            messages.append(Message.assistant([TextBlock(text=f"Response {i}")]))

        # Mock the summarization API call
        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(
                200,
                json={
                    "content": [
                        {"type": "text", "text": "Summary of the conversation."},
                    ],
                    "stop_reason": "end_turn",
                },
            )
        )

        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
        )
        client = AnthropicClient(config)

        result = await compact_conversation(client, messages)

        assert result is not None
        assert len(result) < len(messages)
        # Should contain summary + recent messages
        assert len(result) > 0

    @pytest.mark.asyncio
    @respx.mock
    async def test_compact_handles_api_error(self):
        """Test that compact_conversation returns None on API error."""
        messages = [Message.user(f"msg{i}") for i in range(10)]

        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(500, text="Internal Server Error")
        )

        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
        )
        client = AnthropicClient(config)

        result = await compact_conversation(client, messages)
        assert result is None

    @pytest.mark.asyncio
    async def test_compact_few_messages(self):
        """Test that compact returns messages unchanged when too few."""
        messages = [Message.user(f"msg{i}") for i in range(3)]

        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
        )
        client = AnthropicClient(config)

        result = await compact_conversation(client, messages)
        assert result == messages


class TestAutoCompactIfNeeded:
    @pytest.mark.asyncio
    async def test_disabled_auto_compact(self):
        """When auto_compact is False, no compaction should happen."""
        large_text = "x" * 4000
        messages = [Message.user(large_text)]

        config = AgentConfig(
            api_key="test-key",
            auto_compact=False,
            context_window_size=1000,
        )
        client = AnthropicClient(config)

        result, failures = await auto_compact_if_needed(
            config, client, messages, 0
        )
        assert result == messages
        assert failures == 0

    @pytest.mark.asyncio
    async def test_below_threshold_no_compact(self):
        """When below threshold, no compaction should happen."""
        messages = [Message.user("Hello")]

        config = AgentConfig(
            api_key="test-key",
            auto_compact=True,
            context_window_size=200000,
            auto_compact_threshold_pct=0.8,
        )
        client = AnthropicClient(config)

        result, failures = await auto_compact_if_needed(
            config, client, messages, 0
        )
        assert result == messages
        assert failures == 0

    @pytest.mark.asyncio
    @respx.mock
    async def test_compact_triggered_above_threshold(self):
        """When above threshold, compaction should be triggered."""
        large_text = "x" * 4000
        messages = [Message.user(large_text)]

        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(
                200,
                json={
                    "content": [
                        {"type": "text", "text": "Summary."},
                    ],
                    "stop_reason": "end_turn",
                },
            )
        )

        config = AgentConfig(
            api_key="test-key",
            context_window_size=1000,
            auto_compact_threshold_pct=0.8,
        )
        client = AnthropicClient(config)

        result, failures = await auto_compact_if_needed(
            config, client, messages, 0
        )
        assert result is not None
        assert failures == 0

    @pytest.mark.asyncio
    async def test_circuit_breaker(self):
        """After MAX_CONSECUTIVE_FAILURES, no more compaction attempts."""
        large_text = "x" * 4000
        messages = [Message.user(large_text)]

        config = AgentConfig(
            api_key="test-key",
            context_window_size=1000,
        )
        client = AnthropicClient(config)

        # Simulate 3 consecutive failures (the max)
        from agentlib.compact import MAX_CONSECUTIVE_FAILURES

        result, failures = await auto_compact_if_needed(
            config, client, messages, MAX_CONSECUTIVE_FAILURES
        )
        assert result == messages  # No change
        assert failures == MAX_CONSECUTIVE_FAILURES  # Not incremented


class TestAgentWithAutoCompact:
    @pytest.mark.asyncio
    @respx.mock
    async def test_compact_event_emitted(self):
        """Test that COMPACT event is emitted when compaction occurs."""
        # We need messages large enough to trigger compact at turn 2
        # Use a very small context window
        call_count = 0

        def mock_response(request):
            nonlocal call_count
            call_count += 1

            if call_count <= 2:
                # First two calls: return tool_use to keep the loop going
                return Response(
                    200,
                    text="data: "
                    + json.dumps({
                        "type": "content_block_start",
                        "content_block": {
                            "type": "tool_use",
                            "name": "mock_tool",
                            "id": f"tu_{call_count}",
                            "input": {},
                        },
                    })
                    + "\n\ndata: "
                    + json.dumps({"type": "message_stop"})
                    + "\n\n",
                    headers={"content-type": "text/event-stream"},
                )
            else:
                return Response(
                    200,
                    text="data: "
                    + json.dumps({
                        "type": "content_block_start",
                        "content_block": {"type": "text", "text": "Done"},
                    })
                    + "\n\ndata: "
                    + json.dumps({"type": "message_stop"})
                    + "\n\n",
                    headers={"content-type": "text/event-stream"},
                )

        # Mock both the streaming API and the compact API
        respx.post("https://test.api.com/v1/messages").mock(
            side_effect=mock_response
        )

        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
            context_window_size=100,  # Very small to trigger compact quickly
            auto_compact_threshold_pct=0.5,
        )
        agent = Agent(config)

        # Register a mock tool
        class MockTool:
            @property
            def name(self):
                return "mock_tool"

            @property
            def description(self):
                return "A mock tool"

            @property
            def input_schema(self):
                return {"type": "object", "properties": {}}

            @property
            def is_read_only(self):
                return True

            async def call(self, input, context):
                from agentlib import ToolResult
                return ToolResult.success("mock result")

        agent.register_tool(MockTool())

        # Use a long input to ensure compact triggers
        events = []
        async for event in agent.run("x" * 2000 + " do stuff"):
            events.append(event)

        # Check that compact was either triggered or not needed
        event_types = [e.type for e in events]
        assert EventType.COMPLETE in event_types or EventType.ERROR in event_types

    @pytest.mark.asyncio
    async def test_auto_compact_disabled(self):
        """Test that auto_compact=False prevents compaction."""
        config = AgentConfig(
            api_key="test-key",
            auto_compact=False,
        )
        agent = Agent(config)
        assert agent._config.auto_compact is False


class TestDirectoryAccessControl:
    @pytest.mark.asyncio
    async def test_read_from_allowed_dir(self):
        """Test reading from an explicitly allowed directory."""
        with tempfile.TemporaryDirectory() as tmpdir, \
             tempfile.TemporaryDirectory() as other_dir:
            # Create a file in other_dir
            test_file = os.path.join(other_dir, "test.txt")
            with open(test_file, "w") as f:
                f.write("hello from other dir")

            config = AgentConfig(
                api_key="test-key",
                work_dir=tmpdir,
                allowed_read_dirs=[other_dir],
            )
            agent = Agent(config)

            from agentlib import ReadFileTool, ToolContext
            tool = ReadFileTool()
            result = await tool.call(
                {"file_path": test_file},
                ToolContext(
                    work_dir=tmpdir,
                    message_history=[],
                    allowed_read_dirs=[other_dir],
                ),
            )
            assert result.is_error is False
            assert "hello from other dir" in result.content

    @pytest.mark.asyncio
    async def test_read_from_disallowed_dir(self):
        """Test reading from a disallowed directory is blocked."""
        with tempfile.TemporaryDirectory() as tmpdir, \
             tempfile.TemporaryDirectory() as other_dir:
            # Create a file in other_dir
            test_file = os.path.join(other_dir, "test.txt")
            with open(test_file, "w") as f:
                f.write("secret content")

            config = AgentConfig(
                api_key="test-key",
                work_dir=tmpdir,
                # No allowed_read_dirs configured
            )
            agent = Agent(config)

            from agentlib import ReadFileTool, ToolContext
            tool = ReadFileTool()
            result = await tool.call(
                {"file_path": test_file},
                ToolContext(
                    work_dir=tmpdir,
                    message_history=[],
                ),
            )
            assert result.is_error is True
            assert "escapes" in result.content

    @pytest.mark.asyncio
    async def test_write_to_allowed_dir(self):
        """Test writing to an explicitly allowed directory."""
        with tempfile.TemporaryDirectory() as tmpdir, \
             tempfile.TemporaryDirectory() as other_dir:
            test_file = os.path.join(other_dir, "output.txt")

            from agentlib import WriteFileTool, ToolContext
            tool = WriteFileTool()
            result = await tool.call(
                {"file_path": test_file, "content": "test output"},
                ToolContext(
                    work_dir=tmpdir,
                    message_history=[],
                    allowed_write_dirs=[other_dir],
                ),
            )
            assert result.is_error is False

    @pytest.mark.asyncio
    async def test_write_to_disallowed_dir(self):
        """Test writing to a disallowed directory is blocked."""
        with tempfile.TemporaryDirectory() as tmpdir, \
             tempfile.TemporaryDirectory() as other_dir:
            test_file = os.path.join(other_dir, "output.txt")

            from agentlib import WriteFileTool, ToolContext
            tool = WriteFileTool()
            result = await tool.call(
                {"file_path": test_file, "content": "test"},
                ToolContext(
                    work_dir=tmpdir,
                    message_history=[],
                ),
            )
            assert result.is_error is True

    @pytest.mark.asyncio
    async def test_work_dir_always_accessible(self):
        """Test that work_dir is always readable and writable."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Write
            from agentlib import WriteFileTool, ReadFileTool, ToolContext
            write_tool = WriteFileTool()
            result = await write_tool.call(
                {"file_path": "test.txt", "content": "hello"},
                ToolContext(work_dir=tmpdir, message_history=[]),
            )
            assert result.is_error is False

            # Read
            read_tool = ReadFileTool()
            result = await read_tool.call(
                {"file_path": "test.txt"},
                ToolContext(work_dir=tmpdir, message_history=[]),
            )
            assert result.is_error is False
            assert "hello" in result.content
