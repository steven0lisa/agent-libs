"""Glob tool for finding files matching a pattern."""

from __future__ import annotations

import glob as glob_module
import os

from ..tool import Tool, ToolContext, ToolResult
from ..utils.security import resolve_safe_path


class GlobTool:
    """Find files matching a pattern."""

    @property
    def name(self) -> str:
        return "glob"

    @property
    def description(self) -> str:
        return "Find files matching a pattern."

    @property
    def is_read_only(self) -> bool:
        return True

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files (e.g. '**/*.py', 'src/**/*.ts')",
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search in (default: current directory)",
                    "default": ".",
                },
            },
            "required": ["pattern"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        pattern = input.get("pattern")
        if not pattern:
            return ToolResult.error("pattern is required")

        search_path = input.get("path", ".")

        # Validate base path
        try:
            resolved_base = resolve_safe_path(
                search_path,
                context.work_dir,
                context.allowed_read_dirs,
            )
        except ValueError as e:
            return ToolResult.error(str(e))

        try:
            # Build full glob pattern
            full_pattern = os.path.join(str(resolved_base), pattern)

            # Execute glob search
            results = glob_module.glob(full_pattern, recursive=True)

            # Filter to only files (not directories), sort results
            files = sorted(f for f in results if os.path.isfile(f))

            if not files:
                return ToolResult.success("No files found matching the pattern.")

            # Make paths relative to work_dir for cleaner output
            work_abs = os.path.abspath(context.work_dir)
            relative_files = []
            for f in files:
                try:
                    rel = os.path.relpath(f, work_abs)
                    # Don't use relative path if it goes outside work_dir
                    if rel.startswith(".."):
                        relative_files.append(f)
                    else:
                        relative_files.append(rel)
                except ValueError:
                    relative_files.append(f)

            output = "\n".join(relative_files)
            return ToolResult.success(output)

        except Exception as e:
            return ToolResult.error(f"Glob search failed: {e}")
