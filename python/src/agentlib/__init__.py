"""AgentLib - A lightweight agent library for Anthropic API."""

from .agent import Agent
from .compact import (
    estimate_tokens,
    should_compact,
    compact_conversation,
    auto_compact_if_needed,
)
from .config import AgentConfig, Pattern, OutputFormat
from .skills import SkillLoader, SkillTool, SkillInfo, SkillMetadata, SkillLoaderOptions
from .tool import Tool, ToolContext, ToolResult
from .types import (
    Message,
    Role,
    ContentBlock,
    TextBlock,
    ToolUseBlock,
    ToolResultBlock,
    ThinkingBlock,
    Event,
    EventType,
)
from .tools import ReadFileTool, WriteFileTool, UpdateFileTool, BashTool, CurlTool, GrepTool, GlobTool
from .tools.subagent import SubAgentTool

__all__ = [
    "Agent",
    "AgentConfig",
    "Pattern",
    "OutputFormat",
    "SkillLoader",
    "SkillLoaderOptions",
    "SkillTool",
    "SkillInfo",
    "SkillMetadata",
    "Tool",
    "ToolContext",
    "ToolResult",
    "Message",
    "Role",
    "ContentBlock",
    "TextBlock",
    "ToolUseBlock",
    "ToolResultBlock",
    "ThinkingBlock",
    "Event",
    "EventType",
    "ReadFileTool",
    "WriteFileTool",
    "UpdateFileTool",
    "BashTool",
    "CurlTool",
    "GrepTool",
    "GlobTool",
    "SubAgentTool",
    "estimate_tokens",
    "should_compact",
    "compact_conversation",
    "auto_compact_if_needed",
]
