"""Bash tool with whitelist/blacklist security."""

from __future__ import annotations

import asyncio

from ..config import Pattern
from ..tool import Tool, ToolContext, ToolResult
from ..utils.security import check_security_policy


class BashTool:
    """Execute a shell command with security policy enforcement."""

    def __init__(
        self,
        whitelist: list[Pattern] | None = None,
        blacklist: list[Pattern] | None = None,
    ) -> None:
        self._whitelist = whitelist or []
        self._blacklist = blacklist or []

    @property
    def name(self) -> str:
        return "bash"

    @property
    def description(self) -> str:
        return "Execute a shell command in the working directory."

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute",
                },
                "description": {
                    "type": "string",
                    "description": "A brief description of what the command does",
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds",
                    "default": 120000,
                },
            },
            "required": ["command"],
        }

    @property
    def is_read_only(self) -> bool:
        return False

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        command = input.get("command", "")
        timeout_ms = input.get("timeout", 120_000)
        is_whitelisted = any(p.matches(command) for p in self._whitelist)

        # Security check - enforced at execution time, not disclosed in prompt
        allowed, reason = check_security_policy(
            command,
            self._whitelist,
            self._blacklist,
            default_allow=True,
        )
        if not allowed:
            return ToolResult.error(f"Command blocked by security policy: {reason}")

        try:
            proc = await asyncio.create_subprocess_shell(
                command,
                cwd=context.work_dir,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )

            try:
                stdout, stderr = await asyncio.wait_for(
                    proc.communicate(), timeout=timeout_ms / 1000
                )
            except asyncio.TimeoutError:
                proc.kill()
                return ToolResult.error(
                    f"Command timed out after {timeout_ms}ms"
                )

            output = stdout.decode("utf-8", errors="replace")
            if stderr:
                output += f"\n[stderr]\n{stderr.decode('utf-8', errors='replace')}"

            return ToolResult(content=output, is_error=(proc.returncode != 0 and not is_whitelisted))
        except Exception as e:
            return ToolResult.error(f"Failed to execute: {e}")
