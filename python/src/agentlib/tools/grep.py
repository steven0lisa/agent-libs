"""Grep tool for searching patterns in files."""

from __future__ import annotations

import os
import re

import aiofiles

from ..tool import Tool, ToolContext, ToolResult
from ..utils.security import resolve_safe_path

MAX_MATCHES = 100


class GrepTool:
    """Search for patterns in files using regular expressions."""

    @property
    def name(self) -> str:
        return "grep"

    @property
    def description(self) -> str:
        return "Search for patterns in files using regular expressions."

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
                    "description": "The regular expression pattern to search for",
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file path to search in (default: current directory)",
                    "default": ".",
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '**/*.py')",
                },
                "include": {
                    "type": "string",
                    "description": "File type filter (e.g. '*.py', '*.ts')",
                },
                "ignore_case": {
                    "type": "boolean",
                    "description": "Case insensitive search",
                    "default": False,
                },
                "line_numbers": {
                    "type": "boolean",
                    "description": "Show line numbers",
                    "default": True,
                },
            },
            "required": ["pattern"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        pattern_str = input.get("pattern")
        if not pattern_str:
            return ToolResult.error("pattern is required")

        search_path = input.get("path", ".")
        glob_pattern = input.get("glob")
        include = input.get("include")
        ignore_case = input.get("ignore_case", False)
        show_line_numbers = input.get("line_numbers", True)

        # Validate path
        try:
            resolved = resolve_safe_path(
                search_path,
                context.work_dir,
                context.allowed_read_dirs,
            )
        except ValueError as e:
            return ToolResult.error(str(e))

        # Compile regex
        try:
            flags = re.IGNORECASE if ignore_case else 0
            regex = re.compile(pattern_str, flags)
        except re.error as e:
            return ToolResult.error(f"Invalid regex pattern: {e}")

        # Build file filter from glob/include
        file_filter = glob_pattern or include or None

        matches: list[str] = []
        total_matches = 0

        try:
            if os.path.isfile(resolved):
                # Search single file
                file_matches = await self._search_file(
                    regex, str(resolved), show_line_numbers
                )
                if file_matches:
                    matches.extend(file_matches)
                    total_matches = len(file_matches)
            elif os.path.isdir(resolved):
                # Walk directory tree
                for root, _dirs, files in os.walk(str(resolved)):
                    for filename in sorted(files):
                        if total_matches >= MAX_MATCHES:
                            break

                        filepath = os.path.join(root, filename)

                        # Apply file filter
                        if file_filter and not self._matches_filter(
                            filepath, file_filter
                        ):
                            continue

                        # Skip binary-like files and hidden dirs
                        if self._should_skip(filepath):
                            continue

                        file_matches = await self._search_file(
                            regex, filepath, show_line_numbers
                        )
                        if file_matches:
                            remaining = MAX_MATCHES - total_matches
                            matches.extend(file_matches[:remaining])
                            total_matches += len(file_matches)

                    if total_matches >= MAX_MATCHES:
                        break
            else:
                return ToolResult.error(f"Path not found: {search_path}")

        except Exception as e:
            return ToolResult.error(f"Search failed: {e}")

        if not matches:
            return ToolResult.success("No matches found.")

        output = "\n".join(matches)
        if total_matches >= MAX_MATCHES:
            output += f"\n\n(Results limited to {MAX_MATCHES} matches)"

        return ToolResult.success(output)

    async def _search_file(
        self, regex: re.Pattern, filepath: str, show_line_numbers: bool
    ) -> list[str]:
        """Search a single file for pattern matches."""
        matches: list[str] = []
        try:
            async with aiofiles.open(filepath, "r", encoding="utf-8") as f:
                for line_no, line in enumerate(f, start=1):
                    if regex.search(line):
                        line_content = line.rstrip("\n")
                        if show_line_numbers:
                            matches.append(f"{filepath}:{line_no}:{line_content}")
                        else:
                            matches.append(f"{filepath}:{line_content}")
        except (UnicodeDecodeError, OSError):
            # Skip binary or unreadable files
            pass
        return matches

    @staticmethod
    def _matches_filter(filepath: str, file_filter: str) -> bool:
        """Check if a file path matches the given filter pattern."""
        import fnmatch

        filename = os.path.basename(filepath)
        # Support both glob patterns like "**/*.py" and simple filters like "*.py"
        if "**" in file_filter or "?" in file_filter:
            return fnmatch.fnmatch(filepath, file_filter)
        return fnmatch.fnmatch(filename, file_filter)

    @staticmethod
    def _should_skip(filepath: str) -> bool:
        """Check if a file should be skipped (hidden dirs, common binary locations)."""
        parts = filepath.split(os.sep)
        # Skip hidden directories and common non-text directories
        skip_dirs = {
            ".git",
            ".svn",
            ".hg",
            "node_modules",
            "__pycache__",
            ".venv",
            "venv",
            ".env",
            ".tox",
            ".mypy_cache",
            ".pytest_cache",
            "dist",
            "build",
            ".next",
            ".nuxt",
            "target",
        }
        return any(part in skip_dirs for part in parts)
