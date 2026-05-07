"""Read file tool."""

from __future__ import annotations

import aiofiles

from ..tool import Tool, ToolContext, ToolResult
from ..utils.security import resolve_safe_path


class ReadFileTool:
    """Read file contents from the working directory."""

    @property
    def name(self) -> str:
        return "read_file"

    @property
    def description(self) -> str:
        return "Read file contents from the working directory. Supports text, images, PDFs."

    @property
    def is_read_only(self) -> bool:
        return True

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file (relative to working directory or absolute)",
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from",
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read",
                },
            },
            "required": ["file_path"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        file_path = input.get("file_path")
        if not file_path:
            return ToolResult.error("file_path is required")

        try:
            resolved = resolve_safe_path(file_path, context.work_dir)
        except ValueError as e:
            return ToolResult.error(str(e))

        try:
            async with aiofiles.open(resolved, "r", encoding="utf-8") as f:
                content = await f.read()
            return ToolResult.success(content)
        except Exception as e:
            return ToolResult.error(f"Failed to read file: {e}")
