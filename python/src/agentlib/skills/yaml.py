"""Minimal YAML frontmatter parser for SKILL.md files.

No external dependencies — pure regex and string parsing.
"""

from __future__ import annotations

import re
from pathlib import Path

from .types import SkillMetadata


def parse_frontmatter(raw: str) -> dict:
    """Parse YAML frontmatter from a raw string.

    Extracts content between ``---`` delimiters at the start of the string.
    Supports simple key: value lines with string, bool, int, and array values.

    Args:
        raw: Raw file content.

    Returns:
        Dictionary of parsed metadata keys/values.
    """
    regex = re.compile(r"^---\n([\s\S]*?)\n---\n?", re.MULTILINE)
    match = regex.match(raw)
    if not match:
        return {}

    yaml_content = match.group(1)
    metadata: dict = {}

    for line in yaml_content.split("\n"):
        trimmed = line.strip()
        if not trimmed or trimmed.startswith("#"):
            continue

        colon_index = trimmed.find(":")
        if colon_index == -1:
            continue

        key = trimmed[:colon_index].strip()
        value_part: str = trimmed[colon_index + 1 :].strip()

        if not key or not value_part:
            continue

        # Handle multi-value array lines (continuation with "- " prefix)
        if value_part.startswith("- "):
            metadata[key] = value_part[2:].split(",")
            continue

        # Remove surrounding quotes
        if (value_part.startswith('"') and value_part.endswith('"')) or (
            value_part.startswith("'") and value_part.endswith("'")
        ):
            value_part = value_part[1:-1]

        # Try boolean
        if value_part == "true":
            metadata[key] = True
            continue
        if value_part == "false":
            metadata[key] = False
            continue

        # Try number
        try:
            num = int(value_part)
            metadata[key] = num
            continue
        except ValueError:
            try:
                num = float(value_part)
                metadata[key] = num
                continue
            except ValueError:
                pass

        metadata[key] = value_part

    # Normalize allowed_tools if it was parsed as an array
    if "allowed_tools" in metadata and isinstance(metadata["allowed_tools"], list):
        arr = metadata["allowed_tools"]
        metadata["allowed_tools"] = [
            x.strip() for s in arr for x in s.split(",")
        ]

    return metadata


def parse_skill_file(file_path: str) -> tuple[dict, str]:
    """Read a SKILL.md file and parse its frontmatter.

    Args:
        file_path: Absolute path to a SKILL.md file.

    Returns:
        Tuple of (metadata_dict, content_string) where content is the
        file body with frontmatter stripped.
    """
    raw = Path(file_path).read_text(encoding="utf-8")
    metadata = parse_frontmatter(raw)

    # Remove frontmatter, keep the rest
    regex = re.compile(r"^---\n[\s\S]*?\n---\n?", re.MULTILINE)
    content = regex.sub("", raw).strip()

    return metadata, content
