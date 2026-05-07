"""Skill system types."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class SkillMetadata:
    """Metadata parsed from the YAML frontmatter of a SKILL.md file."""

    name: str = ""
    description: str = ""
    when_to_use: str = ""
    allowed_tools: list[str] = field(default_factory=list)
    model: str = ""
    context: str = "inline"
    version: str = ""


@dataclass
class SkillInfo:
    """Full skill information including parsed metadata and raw content."""

    metadata: SkillMetadata
    content: str
    file_path: str
    dir_path: str


@dataclass
class SkillLoaderOptions:
    """Options for configuring a SkillLoader instance."""

    skills_dir: str = ""
    include_project_skills: bool = False
    project_dir: str = ""
