"""Anthropic API client."""

from __future__ import annotations

import json
from typing import AsyncIterator

import httpx

from .config import AgentConfig
from .types import Message, ContentBlock, TextBlock, ToolUseBlock, ThinkingBlock


class AnthropicClient:
    """Client for Anthropic Messages API."""

    def __init__(self, config: AgentConfig) -> None:
        self._config = config
        self._client = httpx.AsyncClient(
            base_url=config.base_url,
            headers={
                "x-api-key": config.api_key,
                "anthropic-version": "2023-06-01",
                "Content-Type": "application/json",
            },
            timeout=config.timeout_ms / 1000,
            follow_redirects=True,
        )

    async def stream_messages(
        self,
        messages: list[Message],
        system: str,
        tools: list[dict],
    ) -> AsyncIterator[dict]:
        """Send a streaming request to the Anthropic API."""
        request = {
            "model": self._config.model,
            "messages": [self._message_to_dict(m) for m in messages],
            "system": system,
            "tools": tools,
            "max_tokens": self._config.max_tokens,
            "stream": True,
        }

        async with self._client.stream(
            "POST", "/v1/messages", json=request
        ) as response:
            response.raise_for_status()

            async for line in response.aiter_lines():
                if line.startswith("data: "):
                    data = line[6:]
                    if data == "[DONE]":
                        break
                    try:
                        yield json.loads(data)
                    except json.JSONDecodeError:
                        continue

    async def send_messages(
        self,
        messages: list[Message],
        system: str,
        tools: list[dict],
    ) -> dict:
        """Send a non-streaming request to the Anthropic API."""
        request = {
            "model": self._config.model,
            "messages": [self._message_to_dict(m) for m in messages],
            "system": system,
            "tools": tools,
            "max_tokens": self._config.max_tokens,
            "stream": False,
        }

        response = await self._client.post("/v1/messages", json=request)
        response.raise_for_status()
        return response.json()

    def _message_to_dict(self, message: Message) -> dict:
        return {
            "role": message.role.value,
            "content": [self._block_to_dict(b) for b in message.content],
        }

    def _block_to_dict(self, block: ContentBlock) -> dict:
        if isinstance(block, TextBlock):
            return {"type": "text", "text": block.text}
        elif isinstance(block, ToolUseBlock):
            return {
                "type": "tool_use",
                "name": block.name,
                "id": block.id,
                "input": block.input,
            }
        elif isinstance(block, ThinkingBlock):
            return {"type": "thinking", "thinking": block.thinking}
        else:
            # ToolResultBlock - should not be in API request but handle anyway
            from .types import ToolResultBlock
            if isinstance(block, ToolResultBlock):
                result = {
                    "type": "tool_result",
                    "tool_use_id": block.tool_use_id,
                    "content": block.content,
                }
                if block.is_error is not None:
                    result["is_error"] = block.is_error
                return result
            raise ValueError(f"Unknown block type: {type(block)}")
