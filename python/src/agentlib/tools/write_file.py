"""Write file tool."""

from __future__ import annotations

import os

import aiofiles

from ..tool import Tool, ToolContext, ToolResult
from ..utils.security import resolve_safe_path


class WriteFileTool:
    """Write content to a file."""

    @property
    def name(self) -> str:
        return "write_file"

    @property
    def description(self) -> str:
        return "Write content to a file. Creates if not exists, overwrites if exists."

    @property
    def is_read_only(self) -> bool:
        return False

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "file_path": {"type": "string"},
                "content": {"type": "string"},
            },
            "required": ["file_path", "content"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        file_path = input.get("file_path", "")
        content = input.get("content", "")

        try:
            resolved = resolve_safe_path(file_path, context.work_dir)
        except ValueError as e:
            return ToolResult.error(str(e))

        try:
            os.makedirs(os.path.dirname(resolved), exist_ok=True)
            async with aiofiles.open(resolved, "w", encoding="utf-8") as f:
                await f.write(content)
            return ToolResult.success(f"File written: {resolved}")
        except Exception as e:
            return ToolResult.error(f"Failed to write file: {e}")
