"""Agent core implementation with lifecycle management."""

from __future__ import annotations

import asyncio
import time
from enum import Enum, auto
from typing import AsyncIterator, Optional

from .config import AgentConfig, OutputFormat, Pattern
from .client import AnthropicClient
from .prompt import build_system_prompt
from .skills.loader import SkillLoader
from .skills.tool import SkillTool
from .skills.types import SkillLoaderOptions
from .tool import Tool, ToolContext, ToolResult
from .types import (
    ContentBlock,
    Event,
    EventType,
    Message,
    Role,
    TextBlock,
    ThinkingBlock,
    ToolResultBlock,
    ToolUseBlock,
)
from .utils.security import check_security_policy, resolve_safe_path


class AgentState(Enum):
    """Agent lifecycle states."""

    IDLE = "idle"
    RUNNING = "running"
    PAUSED = "paused"
    STOPPING = "stopping"
    TERMINATED = "terminated"
    COMPLETED = "completed"


class Agent:
    """Agent with lifecycle management (run/pause/resume/stop)."""

    def __init__(self, config: AgentConfig) -> None:
        self._config = config
        self._tools: dict[str, Tool] = {}
        self._message_history: list[Message] = []
        self._turn_count = 0
        self._start_time: Optional[float] = None
        self._skill_loader: SkillLoader | None = None
        self._loaded_skills: list | None = None

        # Lifecycle state
        self._state = AgentState.IDLE
        self._pause_event = asyncio.Event()
        self._pause_event.set()  # Not paused by default
        self._stop_event = asyncio.Event()

        # Register built-in tools
        from .tools import BashTool, CurlTool, ReadFileTool, UpdateFileTool, WriteFileTool

        self.register_tool(ReadFileTool())
        self.register_tool(WriteFileTool())
        self.register_tool(UpdateFileTool())
        self.register_tool(
            BashTool(
                whitelist=config.bash_whitelist,
                blacklist=config.bash_blacklist,
            )
        )
        self.register_tool(
            CurlTool(
                whitelist=config.curl_whitelist,
                blacklist=config.curl_blacklist,
            )
        )

        # Register custom tools
        for tool in config.tools:
            self.register_tool(tool)

        # Register subagent tool if enabled
        if config.enable_subagent:
            from .tools.subagent import SubAgentTool

            self.register_tool(
                SubAgentTool(
                    parent_config=config,
                    parent_history=self._message_history,
                    parent_tools=self._tools,
                )
            )

        # Register skill tool if enabled
        if config.enable_skills:
            loader_opts = SkillLoaderOptions(
                skills_dir=config.skills_dir,
                include_project_skills=config.include_project_skills,
                project_dir=config.skills_project_dir,
            )
            self._skill_loader = SkillLoader(loader_opts)
            self.register_tool(SkillTool(self._skill_loader))

    # ------------------------------------------------------------------
    # State management
    # ------------------------------------------------------------------

    @property
    def state(self) -> AgentState:
        return self._state

    def pause(self) -> None:
        """Pause the agent. The current turn will finish, then pause."""
        if self._state == AgentState.RUNNING:
            self._state = AgentState.PAUSED
            self._pause_event.clear()

    def resume(self) -> None:
        """Resume a paused agent."""
        if self._state == AgentState.PAUSED:
            self._state = AgentState.RUNNING
            self._pause_event.set()

    def stop(self) -> None:
        """Stop the agent. Cannot be resumed."""
        self._state = AgentState.STOPPING
        self._stop_event.set()
        self._pause_event.set()  # Unblock if paused

    def _check_state(self) -> bool:
        """Check if the agent should continue running."""
        if self._state in (AgentState.STOPPING, AgentState.TERMINATED):
            return False
        return True

    async def _wait_if_paused(self) -> None:
        """Block until the agent is resumed or stopped."""
        if self._state == AgentState.PAUSED:
            await self._pause_event.wait()

    # ------------------------------------------------------------------
    # Tool management
    # ------------------------------------------------------------------

    def register_tool(self, tool: Tool) -> None:
        self._tools[tool.name] = tool

    def unregister_tool(self, name: str) -> None:
        self._tools.pop(name, None)

    def list_tools(self) -> list[Tool]:
        return list(self._tools.values())

    # ------------------------------------------------------------------
    # History
    # ------------------------------------------------------------------

    def get_message_history(self) -> list[Message]:
        return list(self._message_history)

    def clear_history(self) -> None:
        self._message_history = []
        self._turn_count = 0

    # ------------------------------------------------------------------
    # Core methods
    # ------------------------------------------------------------------

    async def run(self, input: str) -> AsyncIterator[Event]:
        """Run the agent with the given input."""
        self._message_history.append(Message.user(input))
        async for event in self._run_loop():
            yield event

    async def chat(self, messages: list[Message]) -> AsyncIterator[Event]:
        """Continue the conversation with existing messages."""
        self._message_history.extend(messages)
        async for event in self._run_loop():
            yield event

    # ------------------------------------------------------------------
    # Agent loop
    # ------------------------------------------------------------------

    async def _run_loop(self) -> AsyncIterator[Event]:
        self._state = AgentState.RUNNING
        self._start_time = time.time()
        self._stop_event.clear()
        self._pause_event.set()

        def emit(event: Event) -> Event:
            if self._config.callback:
                self._config.callback(event)
            return event

        # Load skills lazily if skill loader exists
        if self._skill_loader is not None:
            self._loaded_skills = self._skill_loader.discover_all()

        system_prompt = build_system_prompt(
            self._tools,
            self._config.system_prompt,
            enable_subagent=self._config.enable_subagent,
            subagent_max_turns=self._config.subagent_max_turns,
            skills=self._loaded_skills,
        )
        client = AnthropicClient(self._config)

        try:
            while self._turn_count < self._config.max_turns:
                # Check stop
                if not self._check_state():
                    break

                # Wait if paused
                await self._wait_if_paused()
                if not self._check_state():
                    break

                self._turn_count += 1
                yield emit(Event.turn_start(self._turn_count))

                # Check duration limit
                if self._config.max_duration_ms > 0 and self._start_time:
                    elapsed = (time.time() - self._start_time) * 1000
                    if elapsed > self._config.max_duration_ms:
                        yield emit(Event.error("Max duration exceeded"))
                        break

                # Build tool definitions
                tool_defs = [
                    {
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                    }
                    for t in self._tools.values()
                ]

                # Wait if paused before API call
                await self._wait_if_paused()
                if not self._check_state():
                    break

                # Stream API response
                assistant_content: list[ContentBlock] = []
                yield emit(Event.message_start())

                try:
                    async for event in client.stream_messages(
                        self._message_history,
                        system_prompt,
                        tool_defs,
                    ):
                        # Check stop during streaming
                        if not self._check_state():
                            break

                        await self._wait_if_paused()

                        event_type = event.get("type")

                        if event_type == "content_block_start":
                            block = self._parse_block(
                                event["content_block"]
                            )
                            assistant_content.append(block)

                        elif event_type == "content_block_delta":
                            delta = event["delta"]
                            if delta.get("type") == "text_delta":
                                yield emit(Event.message_delta(delta["text"]))
                            elif delta.get("type") == "thinking_delta":
                                yield emit(Event.thinking_delta(
                                    delta["thinking"]
                                ))

                        elif event_type == "message_stop":
                            yield emit(Event.message_end())

                except Exception as e:
                    yield emit(Event.error(f"API error: {e}"))
                    break

                if not self._check_state():
                    break

                # Add assistant message to history
                self._message_history.append(
                    Message.assistant(assistant_content)
                )

                # Extract tool uses
                tool_uses = [
                    b
                    for b in assistant_content
                    if isinstance(b, ToolUseBlock)
                ]

                if not tool_uses:
                    # Task complete
                    final_text = self._extract_text(assistant_content)
                    self._state = AgentState.COMPLETED
                    yield emit(Event.complete(final_text))
                    return

                # Execute tools
                results = await self._execute_tools(tool_uses)

                if not self._check_state():
                    break

                # Add tool results to history
                self._message_history.append(
                    Message(role=Role.USER, content=results)
                )

            # Max turns reached
            if self._turn_count >= self._config.max_turns:
                yield emit(Event.error("Max turns reached"))

        finally:
            if self._state not in (
                AgentState.COMPLETED,
                AgentState.TERMINATED,
            ):
                self._state = AgentState.TERMINATED

    # ------------------------------------------------------------------
    # Tool execution
    # ------------------------------------------------------------------

    async def _execute_tools(
        self, tool_uses: list[ToolUseBlock]
    ) -> list[ContentBlock]:
        # Group by read-only
        read_only: list[ToolUseBlock] = []
        write: list[ToolUseBlock] = []

        for block in tool_uses:
            tool = self._tools.get(block.name)
            if tool and tool.is_read_only:
                read_only.append(block)
            else:
                write.append(block)

        results: list[ContentBlock] = []

        # Concurrent read-only
        if read_only:
            tasks = [
                self._execute_single_tool(b) for b in read_only
            ]
            read_results = await asyncio.gather(
                *tasks, return_exceptions=True
            )
            for r in read_results:
                if isinstance(r, Exception):
                    results.append(
                        ToolResultBlock(
                            tool_use_id="unknown",
                            content=str(r),
                            is_error=True,
                        )
                    )
                else:
                    results.append(r)

        # Sequential write
        for block in write:
            if not self._check_state():
                break
            result = await self._execute_single_tool(block)
            results.append(result)

        return results

    async def _execute_single_tool(
        self, block: ToolUseBlock
    ) -> ContentBlock:
        tool = self._tools.get(block.name)
        if not tool:
            return ToolResultBlock(
                tool_use_id=block.id,
                content=f"Tool not found: {block.name}",
                is_error=True,
            )

        context = ToolContext(
            work_dir=self._config.work_dir,
            message_history=list(self._message_history),
        )

        try:
            result = await tool.call(block.input, context)
        except Exception as e:
            result = ToolResult.error(str(e))

        return ToolResultBlock(
            tool_use_id=block.id,
            content=result.content,
            is_error=result.is_error,
        )

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _extract_text(self, blocks: list[ContentBlock]) -> str:
        return "".join(
            b.text for b in blocks if isinstance(b, TextBlock)
        )

    def _parse_block(self, data: dict) -> ContentBlock:
        block_type = data.get("type")
        if block_type == "text":
            return TextBlock(text=data["text"])
        elif block_type == "tool_use":
            return ToolUseBlock(
                name=data["name"],
                id=data["id"],
                input=data.get("input", {}),
            )
        elif block_type == "thinking":
            return ThinkingBlock(thinking=data["thinking"])
        else:
            raise ValueError(f"Unknown block type: {block_type}")
