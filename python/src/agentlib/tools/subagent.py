"""Subagent tool for creating child agents."""

from __future__ import annotations

from ..agent import Agent
from ..config import AgentConfig
from ..tool import Tool, ToolContext, ToolResult
from ..types import Message


class SubAgentTool:
    """Create a subagent to handle an independent task.

    The subagent forks the parent agent's context (message history, tools,
    working directory) and runs with its own turn budget.
    """

    def __init__(
        self,
        parent_config: AgentConfig,
        parent_history: list[Message],
        parent_tools: dict[str, Tool],
    ) -> None:
        self._parent_config = parent_config
        self._parent_history = parent_history
        self._parent_tools = parent_tools

    @property
    def name(self) -> str:
        return "subagent"

    @property
    def description(self) -> str:
        return (
            "Create a subagent to handle an independent task. "
            "The subagent shares your context but operates independently "
            "with its own tool budget."
        )

    @property
    def is_read_only(self) -> bool:
        return True

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Description of the task for the subagent",
                },
            },
            "required": ["task"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        task = input.get("task", "")
        if not task:
            return ToolResult.error("task is required")

        # Fork parent config with reduced max_turns
        sub_config = AgentConfig(
            api_key=self._parent_config.api_key,
            base_url=self._parent_config.base_url,
            model=self._parent_config.model,
            work_dir=self._parent_config.work_dir,
            max_tokens=self._parent_config.max_tokens,
            max_turns=self._parent_config.subagent_max_turns,
            system_prompt=self._parent_config.system_prompt,
            timeout_ms=self._parent_config.timeout_ms,
            stream=False,  # Subagents run non-streaming for simplicity
            output_format=self._parent_config.output_format,
        )

        # Create subagent with forked context
        subagent = Agent(sub_config)

        # Copy parent's tool registry (except subagent itself to avoid recursion)
        for name, tool in self._parent_tools.items():
            if name != "subagent":
                subagent.register_tool(tool)

        # Copy parent's message history for context
        subagent._message_history = list(self._parent_history)

        # Run subagent
        final_content = ""
        try:
            async for event in subagent.run(task):
                if event.type.value == "complete":
                    final_content = event.data.get("final_content", "")
                elif event.type.value == "error":
                    return ToolResult.error(
                        f"Subagent error: {event.data.get('message', '')}"
                    )
        except Exception as e:
            return ToolResult.error(f"Subagent failed: {e}")

        return ToolResult.success(
            f"Subagent completed. Result:\n{final_content}"
        )
