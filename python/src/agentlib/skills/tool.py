"""SkillTool — Tool Protocol implementation for loading skills."""

from __future__ import annotations

from ..tool import Tool, ToolContext, ToolResult
from .loader import SkillLoader
from .substitution import substitute_variables
from .types import SkillInfo


class SkillTool(Tool):
    """Tool that loads a skill and returns its processed content.

    Registered automatically when agent config has ``enable_skills=True``.
    """

    name = "skill"
    description = (
        "Load a skill and get its instructions. "
        "Skills provide specialized capabilities for specific tasks."
    )
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

        result = _build_skill_result(skill, processed)
        return ToolResult.success(result)


def _build_skill_result(skill: SkillInfo, content: str) -> str:
    """Format a skill's metadata and content into a result string."""
    parts: list[str] = []
    parts.append(f"## Skill: {skill.metadata.name}")

    if skill.metadata.description:
        parts.append(f"Description: {skill.metadata.description}")
    if skill.metadata.when_to_use:
        parts.append(f"When to use: {skill.metadata.when_to_use}")

    parts.append("")
    parts.append(content)
    return "\n".join(parts)
