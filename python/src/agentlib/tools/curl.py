"""Curl tool with whitelist/blacklist security."""

from __future__ import annotations

import httpx

from ..config import Pattern
from ..tool import Tool, ToolContext, ToolResult
from ..utils.security import check_security_policy


class CurlTool:
    """Make an HTTP request with security policy enforcement."""

    def __init__(
        self,
        whitelist: list[Pattern] | None = None,
        blacklist: list[Pattern] | None = None,
    ) -> None:
        self._whitelist = whitelist or []
        self._blacklist = blacklist or []
        self._client = httpx.AsyncClient(
            follow_redirects=True,
            verify=True,
        )

    @property
    def name(self) -> str:
        return "curl"

    @property
    def description(self) -> str:
        return "Make an HTTP request."

    @property
    def is_read_only(self) -> bool:
        return True

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "url": {"type": "string"},
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"],
                    "default": "GET",
                },
                "headers": {
                    "type": "object",
                    "additionalProperties": {"type": "string"},
                },
                "body": {"type": "string"},
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in milliseconds",
                    "default": 30000,
                },
            },
            "required": ["url"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        url = input.get("url", "")
        method = input.get("method", "GET")
        body = input.get("body")
        headers = input.get("headers", {})
        timeout = input.get("timeout", 30_000)

        # Security check - enforced at execution time, not disclosed in prompt
        allowed, reason = check_security_policy(
            url,
            self._whitelist,
            self._blacklist,
            default_allow=True,
        )
        if not allowed:
            return ToolResult.error(f"URL blocked by security policy: {reason}")

        try:
            response = await self._client.request(
                method=method,
                url=url,
                headers=headers,
                content=body,
                timeout=httpx.Timeout(timeout / 1000),
            )
            return ToolResult.success(response.text)
        except Exception as e:
            return ToolResult.error(f"HTTP error: {e}")
