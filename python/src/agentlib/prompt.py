"""System prompt builder."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .tool import Tool
    from .skills.types import SkillInfo


DEFAULT_SYSTEM_PROMPT = """You are a helpful software engineering assistant. You have access to tools that let you interact with the file system and execute commands.

## Tool Use

You will be provided with a set of tools. When you need to perform an action, use the appropriate tool by outputting a tool_use block.

Tool use format:
- Each tool has a name and input parameters defined by a JSON schema
- To use a tool, output a content block with type "tool_use"
- The tool_use block must include: name (tool name), id (unique identifier), input (parameters as JSON object)

After using a tool, the user message will contain a tool_result block with the execution result.
- If the result indicates success, continue with your analysis or next steps
- If the result indicates an error, analyze the error and decide whether to retry, use a different approach, or ask the user for clarification

## File Operations

When working with files:
- Always read a file before editing it
- When editing files, use update_file tool with old_string/new_string for precise changes
- Use write_file to create new files
- Use read_file to inspect file contents
- All file operations are restricted to the working directory and its subdirectories

## Command Execution

When executing commands via bash:
- Prefer read-only commands for exploration (ls, grep, find, cat, etc.)
- Be careful with destructive commands (rm, dd, etc.)
- Always describe what the command does before executing it
"""


SUBAGENT_PROMPT = """

## Subagent Capability

You can create subagents to handle independent tasks in parallel. A subagent is a separate agent instance that shares your context but operates independently.

When to use subagents:
- When you need to explore multiple approaches simultaneously
- When a task can be cleanly decomposed into independent sub-tasks
- When you want to parallelize read-only exploration

How to create a subagent:
- Use the `subagent` tool with a `task` describing what the subagent should do
- The subagent will execute with its own tool budget (max turns: {max_turns})
- The subagent inherits your current context (files read, tool state, etc.)
- Results are returned as tool_result when the subagent completes

Important:
- Subagents run independently and do not modify your state
- File operations in subagents are still restricted to the working directory
- Prefer subagents for exploration; keep modifications in the main agent
"""


def build_system_prompt(
    tools: dict[str, "Tool"],
    custom_prompt: str | None = None,
    enable_subagent: bool = False,
    subagent_max_turns: int = 50,
    skills: list | None = None,
) -> str:
    """Build the system prompt with tool descriptions."""
    tool_descriptions = []
    for tool in tools.values():
        tool_descriptions.append(
            f"### {tool.name}\n"
            f"Description: {tool.description}\n"
            f"Input Schema: {tool.input_schema}"
        )

    tools_section = "\n\n## Available Tools\n\n" + "\n\n".join(tool_descriptions)

    prompt = DEFAULT_SYSTEM_PROMPT + tools_section

    if enable_subagent:
        prompt += SUBAGENT_PROMPT.format(max_turns=subagent_max_turns)

    if skills:
        prompt += "\n\n## Available Skills\n\n"
        for s in skills:
            prompt += f"### {s.metadata.name}\n"
            prompt += f"Description: {s.metadata.description}\n"
            if s.metadata.when_to_use:
                prompt += f"When to use: {s.metadata.when_to_use}\n"
        prompt += (
            "\n\nTo use a skill, call the `skill` tool with the "
            "skill name as the `skill` parameter. "
            "You can also pass optional arguments via the `args` parameter."
        )

    if custom_prompt:
        prompt += f"\n\n## Custom Instructions\n\n{custom_prompt}"

    return prompt
