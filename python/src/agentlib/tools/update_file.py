"""Update file tool."""

from __future__ import annotations

import aiofiles

from ..tool import Tool, ToolContext, ToolResult
from ..utils.security import resolve_safe_path


class UpdateFileTool:
    """Update a file by replacing old_string with new_string."""

    @property
    def name(self) -> str:
        return "update_file"

    @property
    def description(self) -> str:
        return "Update a file by replacing old_string with new_string."

    @property
    def is_read_only(self) -> bool:
        return False

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "file_path": {"type": "string"},
                "old_string": {
                    "type": "string",
                    "description": "The text to replace",
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text",
                },
                "replace_all": {
                    "type": "boolean",
                    "default": False,
                    "description": "Replace all occurrences",
                },
            },
            "required": ["file_path", "old_string", "new_string"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        file_path = input.get("file_path", "")
        old_str = input.get("old_string", "")
        new_str = input.get("new_string", "")
        replace_all = input.get("replace_all", False)

        try:
            resolved = resolve_safe_path(
                file_path,
                context.work_dir,
                context.allowed_write_dirs,
            )
        except ValueError as e:
            return ToolResult.error(str(e))

        try:
            async with aiofiles.open(resolved, "r", encoding="utf-8") as f:
                content = await f.read()

            if replace_all:
                new_content = content.replace(old_str, new_str)
            else:
                new_content = content.replace(old_str, new_str, 1)

            if new_content == content:
                return ToolResult.error("old_string not found in file")

            async with aiofiles.open(resolved, "w", encoding="utf-8") as f:
                await f.write(new_content)

            return ToolResult.success(f"File updated: {resolved}")
        except Exception as e:
            return ToolResult.error(f"Failed to update file: {e}")
