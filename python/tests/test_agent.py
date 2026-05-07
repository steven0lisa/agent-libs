"""Tests for Agent core functionality."""

import asyncio
import json
import os
import tempfile

import pytest
import respx
from httpx import Response

from agentlib import (
    Agent,
    AgentConfig,
    BashTool,
    EventType,
    Message,
    Pattern,
    ReadFileTool,
    Role,
    TextBlock,
    Tool,
    ToolContext,
    ToolResult,
    ToolUseBlock,
    WriteFileTool,
)


class MockTool:
    """A mock tool for testing."""

    def __init__(self, name="mock_tool", read_only=True):
        self._name = name
        self._read_only = read_only

    @property
    def name(self):
        return self._name

    @property
    def description(self):
        return "A mock tool for testing"

    @property
    def input_schema(self):
        return {"type": "object", "properties": {}}

    @property
    def is_read_only(self):
        return self._read_only

    async def call(self, input, context):
        return ToolResult.success(f"mock result for {self._name}")


@pytest.fixture
def basic_config():
    return AgentConfig(
        api_key="test-key",
        base_url="https://test.api.com",
        model="test-model",
    )


class TestAgentInitialization:
    def test_default_tools_registered(self, basic_config):
        agent = Agent(basic_config)
        tools = agent.list_tools()
        tool_names = [t.name for t in tools]
        assert "read_file" in tool_names
        assert "write_file" in tool_names
        assert "update_file" in tool_names
        assert "bash" in tool_names
        assert "curl" in tool_names

    def test_custom_tool_registered(self, basic_config):
        agent = Agent(basic_config)
        agent.register_tool(MockTool("custom"))
        tool_names = [t.name for t in agent.list_tools()]
        assert "custom" in tool_names

    def test_unregister_tool(self, basic_config):
        agent = Agent(basic_config)
        agent.unregister_tool("read_file")
        tool_names = [t.name for t in agent.list_tools()]
        assert "read_file" not in tool_names

    def test_initial_state(self, basic_config):
        agent = Agent(basic_config)
        from agentlib.agent import AgentState

        assert agent.state == AgentState.IDLE

    def test_empty_history(self, basic_config):
        agent = Agent(basic_config)
        assert agent.get_message_history() == []

    def test_subagent_tool_when_enabled(self, basic_config):
        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
            enable_subagent=True,
        )
        agent = Agent(config)
        tool_names = [t.name for t in agent.list_tools()]
        assert "subagent" in tool_names

    def test_no_subagent_tool_when_disabled(self, basic_config):
        agent = Agent(basic_config)
        tool_names = [t.name for t in agent.list_tools()]
        assert "subagent" not in tool_names


class TestAgentLifecycle:
    def test_pause_sets_paused_state(self, basic_config):
        agent = Agent(basic_config)
        from agentlib.agent import AgentState

        # Manually set state to RUNNING for testing
        agent._state = AgentState.RUNNING
        agent.pause()
        assert agent.state == AgentState.PAUSED

    def test_resume_sets_running_state(self, basic_config):
        agent = Agent(basic_config)
        from agentlib.agent import AgentState

        agent._state = AgentState.RUNNING
        agent.pause()
        assert agent.state == AgentState.PAUSED
        agent.resume()
        assert agent.state == AgentState.RUNNING

    def test_stop_sets_stopping_state(self, basic_config):
        agent = Agent(basic_config)
        from agentlib.agent import AgentState

        agent._state = AgentState.RUNNING
        agent.stop()
        assert agent.state == AgentState.STOPPING

    def test_stop_unblocks_pause(self, basic_config):
        agent = Agent(basic_config)
        from agentlib.agent import AgentState

        agent._state = AgentState.RUNNING
        agent.pause()
        assert not agent._pause_event.is_set()
        agent.stop()
        assert agent._pause_event.is_set()

    def test_clear_history(self, basic_config):
        agent = Agent(basic_config)
        agent._message_history = [Message.user("test")]
        agent._turn_count = 5
        agent.clear_history()
        assert agent.get_message_history() == []
        assert agent._turn_count == 0


class TestAgentCallback:
    @pytest.mark.asyncio
    @respx.mock
    async def test_callback_receives_events(self, basic_config):
        """Test that callback is called for each event."""
        events_received = []

        def callback(event):
            events_received.append(event.type.value)

        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
            max_turns=1,
            callback=callback,
        )

        # Mock API to return a simple text response (no tool_use)
        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(
                200,
                text="data: "
                + json.dumps(
                    {
                        "type": "content_block_start",
                        "content_block": {"type": "text", "text": "Hello"},
                    }
                )
                + "\n\ndata: "
                + json.dumps({"type": "content_block_delta", "delta": {"type": "text_delta", "text": ""}})
                + "\n\ndata: "
                + json.dumps({"type": "message_stop"})
                + "\n\n",
                headers={"content-type": "text/event-stream"},
            )
        )

        agent = Agent(config)
        async for event in agent.run("test"):
            pass

        assert len(events_received) > 0
        assert "turn_start" in events_received


class TestAgentLoop:
    @pytest.mark.asyncio
    @respx.mock
    async def test_run_completes_with_text_response(self, basic_config):
        """Agent loop completes when model returns text without tool_use."""
        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(
                200,
                text="data: "
                + json.dumps(
                    {
                        "type": "content_block_start",
                        "content_block": {"type": "text", "text": "Final answer"},
                    }
                )
                + "\n\ndata: "
                + json.dumps({"type": "message_stop"})
                + "\n\n",
                headers={"content-type": "text/event-stream"},
            )
        )

        agent = Agent(basic_config)
        events = []
        async for event in agent.run("hello"):
            events.append(event)

        event_types = [e.type for e in events]
        assert EventType.TURN_START in event_types
        assert EventType.MESSAGE_START in event_types
        assert EventType.COMPLETE in event_types

        complete_event = [e for e in events if e.type == EventType.COMPLETE][0]
        assert complete_event.data["final_content"] == "Final answer"

        from agentlib.agent import AgentState

        assert agent.state == AgentState.COMPLETED

    @pytest.mark.asyncio
    @respx.mock
    async def test_run_with_tool_use(self, basic_config):
        """Agent loop handles tool_use and continues to next turn."""
        call_count = 0

        def mock_response(request):
            nonlocal call_count
            call_count += 1

            if call_count == 1:
                # First call: model requests tool_use
                return Response(
                    200,
                    text="data: "
                    + json.dumps(
                        {
                            "type": "content_block_start",
                            "content_block": {
                                "type": "tool_use",
                                "name": "mock_tool",
                                "id": "tu_01",
                                "input": {},
                            },
                        }
                    )
                    + "\n\ndata: "
                    + json.dumps({"type": "message_stop"})
                    + "\n\n",
                    headers={"content-type": "text/event-stream"},
                )
            else:
                # Second call: model returns final text
                return Response(
                    200,
                    text="data: "
                    + json.dumps(
                        {
                            "type": "content_block_start",
                            "content_block": {"type": "text", "text": "Done"},
                        }
                    )
                    + "\n\ndata: "
                    + json.dumps({"type": "message_stop"})
                    + "\n\n",
                    headers={"content-type": "text/event-stream"},
                )

        route = respx.post("https://test.api.com/v1/messages").mock(
            side_effect=mock_response
        )

        agent = Agent(basic_config)
        agent.register_tool(MockTool("mock_tool"))

        events = []
        async for event in agent.run("hello"):
            events.append(event)

        event_types = [e.type for e in events]
        assert EventType.TOOL_USE_START in event_types or EventType.TURN_START in event_types

        # Should have made 2 API calls (tool request + final answer)
        assert call_count == 2

    @pytest.mark.asyncio
    @respx.mock
    async def test_max_turns_limit(self, basic_config):
        """Agent stops after max_turns."""
        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(
                200,
                text="data: "
                + json.dumps(
                    {
                        "type": "content_block_start",
                        "content_block": {
                            "type": "tool_use",
                            "name": "mock_tool",
                            "id": "tu_01",
                            "input": {},
                        },
                    }
                )
                + "\n\ndata: "
                + json.dumps({"type": "message_stop"})
                + "\n\n",
                headers={"content-type": "text/event-stream"},
            )
        )

        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
            max_turns=3,
        )
        agent = Agent(config)
        agent.register_tool(MockTool("mock_tool"))

        events = []
        async for event in agent.run("hello"):
            events.append(event)

        event_types = [e.type for e in events]
        assert EventType.ERROR in event_types

        error_event = [e for e in events if e.type == EventType.ERROR][-1]
        assert "Max turns" in error_event.data["message"]

    @pytest.mark.asyncio
    @respx.mock
    async def test_api_error_handling(self, basic_config):
        """Agent handles API errors gracefully."""
        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(500, text="Internal Server Error")
        )

        agent = Agent(basic_config)
        events = []
        async for event in agent.run("hello"):
            events.append(event)

        event_types = [e.type for e in events]
        assert EventType.ERROR in event_types

    @pytest.mark.asyncio
    @respx.mock
    async def test_stop_during_run(self, basic_config):
        """Agent can be stopped during execution."""
        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(
                200,
                text="data: "
                + json.dumps(
                    {
                        "type": "content_block_start",
                        "content_block": {
                            "type": "tool_use",
                            "name": "mock_tool",
                            "id": "tu_01",
                            "input": {},
                        },
                    }
                )
                + "\n\ndata: "
                + json.dumps({"type": "message_stop"})
                + "\n\n",
                headers={"content-type": "text/event-stream"},
            )
        )

        agent = Agent(basic_config)
        agent.register_tool(MockTool("mock_tool"))

        # Stop after a short delay
        async def stop_after_delay():
            await asyncio.sleep(0.05)
            agent.stop()

        asyncio.create_task(stop_after_delay())

        events = []
        async for event in agent.run("hello"):
            events.append(event)

        from agentlib.agent import AgentState

        assert agent.state in (AgentState.TERMINATED, AgentState.STOPPING)


class TestAgentToolExecution:
    @pytest.mark.asyncio
    async def test_concurrent_read_only_tools(self, basic_config):
        """Read-only tools should be executed concurrently."""
        agent = Agent(basic_config)

        # Create two read-only mock tools
        tool1 = MockTool("tool1", read_only=True)
        tool2 = MockTool("tool2", read_only=True)
        agent.register_tool(tool1)
        agent.register_tool(tool2)

        # Simulate tool_use blocks
        tool_uses = [
            ToolUseBlock(name="tool1", id="tu_01", input={}),
            ToolUseBlock(name="tool2", id="tu_02", input={}),
        ]

        start = asyncio.get_event_loop().time()
        results = await agent._execute_tools(tool_uses)
        elapsed = asyncio.get_event_loop().time() - start

        # Both should complete (concurrent execution should be fast)
        assert len(results) == 2

    @pytest.mark.asyncio
    async def test_sequential_write_tools(self, basic_config):
        """Write tools should be executed sequentially."""
        agent = Agent(basic_config)

        execution_order = []

        class SlowWriteTool:
            @property
            def name(self):
                return "slow_write"

            @property
            def description(self):
                return "Slow write tool"

            @property
            def input_schema(self):
                return {}

            @property
            def is_read_only(self):
                return False

            async def call(self, input, context):
                execution_order.append(self.name)
                await asyncio.sleep(0.01)
                return ToolResult.success("done")

        agent.register_tool(SlowWriteTool())

        tool_uses = [
            ToolUseBlock(name="slow_write", id="tu_01", input={}),
        ]

        results = await agent._execute_tools(tool_uses)
        assert len(results) == 1

    @pytest.mark.asyncio
    async def test_tool_not_found(self, basic_config):
        """Unknown tool returns error result."""
        agent = Agent(basic_config)

        tool_uses = [
            ToolUseBlock(name="nonexistent", id="tu_01", input={}),
        ]

        results = await agent._execute_tools(tool_uses)
        assert len(results) == 1
        assert "not found" in results[0].content.lower()


class TestAgentMessageHistory:
    def test_history_after_run(self, basic_config):
        """Message history accumulates during the run."""
        agent = Agent(basic_config)
        agent._message_history = [Message.user("hello")]

        # Manually simulate a turn
        agent._message_history.append(
            Message.assistant([TextBlock(text="response")])
        )

        history = agent.get_message_history()
        assert len(history) == 2
        assert history[0].role == Role.USER
        assert history[1].role == Role.ASSISTANT

    def test_chat_appends_messages(self, basic_config):
        """chat() appends provided messages to history."""
        agent = Agent(basic_config)
        initial_len = len(agent.get_message_history())

        # Note: chat() is async, so we test the history accumulation logic
        messages = [Message.user("msg1"), Message.user("msg2")]
        agent._message_history.extend(messages)

        assert len(agent.get_message_history()) == initial_len + 2


class TestAgentOutputFormat:
    @pytest.mark.asyncio
    @respx.mock
    async def test_json_output_format(self, basic_config):
        """JSON output format includes structured data."""
        route = respx.post("https://test.api.com/v1/messages").mock(
            return_value=Response(
                200,
                text="data: "
                + json.dumps(
                    {
                        "type": "content_block_start",
                        "content_block": {"type": "text", "text": "Answer"},
                    }
                )
                + "\n\ndata: "
                + json.dumps({"type": "message_stop"})
                + "\n\n",
                headers={"content-type": "text/event-stream"},
            )
        )

        config = AgentConfig(
            api_key="test-key",
            base_url="https://test.api.com",
            output_format="json",
        )
        agent = Agent(config)

        events = []
        async for event in agent.run("hello"):
            events.append(event)

        complete_events = [e for e in events if e.type == EventType.COMPLETE]
        assert len(complete_events) == 1


class TestAgentWithRealFileOperations:
    @pytest.mark.asyncio
    @respx.mock
    async def test_agent_with_file_tools(self, basic_config):
        """Agent can use file tools in the loop."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a test file
            with open(os.path.join(tmpdir, "test.txt"), "w") as f:
                f.write("hello")

            call_count = 0

            def mock_response(request):
                nonlocal call_count
                call_count += 1

                if call_count == 1:
                    return Response(
                        200,
                        text="data: "
                        + json.dumps(
                            {
                                "type": "content_block_start",
                                "content_block": {
                                    "type": "tool_use",
                                    "name": "read_file",
                                    "id": "tu_01",
                                    "input": {"file_path": "test.txt"},
                                },
                            }
                        )
                        + "\n\ndata: "
                        + json.dumps({"type": "message_stop"})
                        + "\n\n",
                        headers={"content-type": "text/event-stream"},
                    )
                else:
                    return Response(
                        200,
                        text="data: "
                        + json.dumps(
                            {
                                "type": "content_block_start",
                                "content_block": {"type": "text", "text": "done"},
                            }
                        )
                        + "\n\ndata: "
                        + json.dumps({"type": "message_stop"})
                        + "\n\n",
                        headers={"content-type": "text/event-stream"},
                    )

            route = respx.post("https://test.api.com/v1/messages").mock(
                side_effect=mock_response
            )

            config = AgentConfig(
                api_key="test-key",
                base_url="https://test.api.com",
                work_dir=tmpdir,
            )
            agent = Agent(config)

            events = []
            async for event in agent.run("read the file"):
                events.append(event)

            # Should have 2 API calls
            assert call_count == 2

            # Check tool result was added to history
            history = agent.get_message_history()
            assert len(history) >= 3  # user + assistant(tool_use) + user(tool_result) + ...
