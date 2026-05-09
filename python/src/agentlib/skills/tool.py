"""SkillTool — Tool Protocol implementation for loading skills."""

from __future__ import annotations

from ..tool import Tool, ToolContext, ToolResult
from ..types import ContentBlock, Message, Role, TextBlock
from .loader import SkillLoader
from .substitution import substitute_variables
from .types import SkillInfo


class SkillTool(Tool):
    """Tool that loads a skill and returns its processed content.

    Registered automatically when agent config has ``enable_skills=True``.
    """

    name = "skill"
    is_read_only = True

    input_schema = {
        "type": "object",
        "properties": {
            "skill": {
                "type": "string",
                "description": "The name of the skill to load",
            },
            "args": {
                "type": "string",
                "description": (
                    "Optional arguments passed to the skill via "
                    "$ARGUMENTS variable substitution"
                ),
            },
        },
        "required": ["skill"],
    }

    def __init__(self, loader: SkillLoader) -> None:
        self._loader = loader

    @property
    def description(self) -> str:
        desc = (
            "Load a skill and get its instructions. "
            "Skills provide specialized capabilities for specific tasks."
        )
        skills = self._loader.discover_all()
        invocable = [s for s in skills if s.metadata.user_invocable]
        if invocable:
            desc += "\n\nAvailable skills:\n"
            for skill in invocable:
                if skill.metadata.description:
                    desc += f"- {skill.metadata.name}: {skill.metadata.description}\n"
                else:
                    desc += f"- {skill.metadata.name}\n"
        return desc

    async def call(
        self, input: dict, context: ToolContext
    ) -> ToolResult:
        skill_name = input.get("skill", "")
        args = input.get("args")

        if not skill_name:
            return ToolResult.error(
                '"skill" is required'
            )

        skill = self._loader.find_by_name(skill_name)
        if not skill:
            available = self._loader.discover_all()
            names = [s.metadata.name for s in available]
            names_str = ", ".join(names) if names else "(none)"
            return ToolResult.error(
                f'Skill not found: "{skill_name}". '
                f"Available skills: {names_str}"
            )

        processed = substitute_variables(
            skill.content,
            args=args,
            skill_dir=skill.dir_path,
        )

        full_content = _build_skill_injection_content(skill, processed)
        brief = f"Skill loaded: {skill.metadata.name}"
        injection_msg = Message(
            role=Role.USER,
            content=[TextBlock(text=full_content)],
        )
        return ToolResult.success_with_messages(brief, [injection_msg])


def _build_skill_injection_content(skill: SkillInfo, content: str) -> str:
    """Format a skill's metadata and content into an injection string."""
    parts: list[str] = []
    parts.append(f"## Skill: {skill.metadata.name}")

    if skill.metadata.description:
        parts.append(f"Description: {skill.metadata.description}")
    if skill.metadata.when_to_use:
        parts.append(f"When to use: {skill.metadata.when_to_use}")

    parts.append("")
    parts.append(content)

    if skill.metadata.allowed_tools:
        parts.append("")
        parts.append(
            f"Note: When following this skill's instructions, "
            f"only use these tools: {', '.join(skill.metadata.allowed_tools)}"
        )

    if skill.metadata.context == "fork":
        parts.append("")
        parts.append("This skill should be executed in a fork context.")

    return "\n".join(parts)
