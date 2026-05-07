# PRD - Python 实现

## 1. 语言特性映射

| 通用概念 | Python 实现 |
|----------|-------------|
| Agent | `class Agent` |
| Tool | `Protocol Tool` / `dataclass Tool` / `class BaseTool` |
| Message | `dataclass Message` + `enum Role` |
| ContentBlock | `TypedDict` / `dataclass` / `@dataclass(frozen=True)` |
| Event | `dataclass Event` + `enum EventType` |
| Config | `dataclass AgentConfig` |
| 异步 | `async`/`await` + `asyncio` |
| 事件流 | `AsyncIterator[Event]` 或 `asyncio.Queue` |
| 错误处理 | 异常（`AgentError`） |
| JSON | `dict` / `pydantic.BaseModel` |
| HTTP | `httpx` 或 `aiohttp` |

**最低 Python 版本**: 3.10（使用 `|` union type syntax）

## 2. 核心类型设计

### 2.1 ContentBlock

```python
from dataclasses import dataclass, field
from typing import Optional
from enum import Enum, auto

@dataclass(frozen=True)
class TextBlock:
    type: str = field(default="text", init=False)
    text: str

@dataclass(frozen=True)
class ToolUseBlock:
    type: str = field(default="tool_use", init=False)
    name: str
    id: str
    input: dict

@dataclass(frozen=True)
class ToolResultBlock:
    type: str = field(default="tool_result", init=False)
    tool_use_id: str
    content: str
    is_error: Optional[bool] = None

@dataclass(frozen=True)
class ThinkingBlock:
    type: str = field(default="thinking", init=False)
    thinking: str
    signature: Optional[str] = None

ContentBlock = TextBlock | ToolUseBlock | ToolResultBlock | ThinkingBlock
```

### 2.2 Message

```python
from dataclasses import dataclass
from enum import Enum

class Role(str, Enum):
    USER = "user"
    ASSISTANT = "assistant"

@dataclass
class Message:
    role: Role
    content: list[ContentBlock]

    @staticmethod
    def user(text: str) -> "Message":
        return Message(role=Role.USER, content=[TextBlock(text=text)])

    @staticmethod
    def assistant(blocks: list[ContentBlock]) -> "Message":
        return Message(role=Role.ASSISTANT, content=blocks)
```

### 2.3 Tool Protocol / ABC

```python
from typing import Protocol, runtime_checkable
from dataclasses import dataclass

@dataclass
class ToolContext:
    work_dir: str
    message_history: list[Message]

@dataclass
class ToolResult:
    content: str
    is_error: bool = False

    @staticmethod
    def success(content: str) -> "ToolResult":
        return ToolResult(content=content, is_error=False)

    @staticmethod
    def error(content: str) -> "ToolResult":
        return ToolResult(content=content, is_error=True)

@runtime_checkable
class Tool(Protocol):
    @property
    def name(self) -> str: ...

    @property
    def description(self) -> str: ...

    @property
    def input_schema(self) -> dict: ...

    @property
    def is_read_only(self) -> bool:
        return False

    async def call(self, input: dict, context: ToolContext) -> ToolResult: ...
```

### 2.4 Event

```python
from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

class EventType(str, Enum):
    TURN_START = "turn_start"
    MESSAGE_START = "message_start"
    MESSAGE_DELTA = "message_delta"
    THINKING_DELTA = "thinking_delta"
    MESSAGE_END = "message_end"
    TOOL_USE_START = "tool_use_start"
    TOOL_USE_END = "tool_use_end"
    ERROR = "error"
    COMPLETE = "complete"

@dataclass
class Event:
    type: EventType
    data: dict

    @staticmethod
    def turn_start(turn: int) -> "Event":
        return Event(type=EventType.TURN_START, data={"turn": turn})

    @staticmethod
    def message_delta(text: str) -> "Event":
        return Event(type=EventType.MESSAGE_DELTA, data={"text": text})

    @staticmethod
    def thinking_delta(thinking: str) -> "Event":
        return Event(type=EventType.THINKING_DELTA, data={"thinking": thinking})

    @staticmethod
    def tool_use_start(name: str, id: str, input: dict) -> "Event":
        return Event(type=EventType.TOOL_USE_START, data={"name": name, "id": id, "input": input})

    @staticmethod
    def tool_use_end(name: str, id: str, result: ToolResult) -> "Event":
        return Event(type=EventType.TOOL_USE_END, data={"name": name, "id": id, "result": result})

    @staticmethod
    def error(message: str) -> "Event":
        return Event(type=EventType.ERROR, data={"message": message})

    @staticmethod
    def complete(final_content: str) -> "Event":
        return Event(type=EventType.COMPLETE, data={"final_content": final_content})
```

### 2.5 AgentConfig

```python
from dataclasses import dataclass, field
from typing import Optional
import os

@dataclass
class AgentConfig:
    api_key: str
    base_url: str = "https://api.anthropic.com"
    model: str = "claude-sonnet-4-6"
    work_dir: str = field(default_factory=lambda: os.getcwd())
    max_tokens: int = 8192
    max_turns: int = 100
    system_prompt: Optional[str] = None
    timeout_ms: int = 120_000
    stream: bool = True
    tools: list[Tool] = field(default_factory=list)
```

### 2.6 Agent

```python
from typing import AsyncIterator

class Agent:
    def __init__(self, config: AgentConfig):
        self._config = config
        self._tools: dict[str, Tool] = {}
        self._message_history: list[Message] = []
        self._turn_count = 0

        # 注册预定义工具
        self.register_tool(ReadFileTool())
        self.register_tool(WriteFileTool())
        self.register_tool(UpdateFileTool())
        self.register_tool(BashTool())
        self.register_tool(CurlTool())

        # 注册自定义工具
        for tool in config.tools:
            self.register_tool(tool)

    def register_tool(self, tool: Tool) -> None:
        self._tools[tool.name] = tool

    def unregister_tool(self, name: str) -> None:
        self._tools.pop(name, None)

    def list_tools(self) -> list[Tool]:
        return list(self._tools.values())

    async def run(self, input: str) -> AsyncIterator[Event]:
        """运行 agent loop，返回异步事件流"""
        self._message_history.append(Message.user(input))
        async for event in self._run_loop():
            yield event

    async def chat(self, messages: list[Message]) -> AsyncIterator[Event]:
        """以已有消息历史继续对话"""
        self._message_history.extend(messages)
        async for event in self._run_loop():
            yield event

    def get_message_history(self) -> list[Message]:
        return list(self._message_history)

    def clear_history(self) -> None:
        self._message_history = []
        self._turn_count = 0
```

## 3. 预定义工具实现

### 3.1 ReadFileTool

```python
import aiofiles
import os

class ReadFileTool:
    @property
    def name(self) -> str:
        return "read_file"

    @property
    def description(self) -> str:
        return "Read file contents from the working directory."

    @property
    def is_read_only(self) -> bool:
        return True

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "Path to the file"},
                "offset": {"type": "integer"},
                "limit": {"type": "integer"},
            },
            "required": ["file_path"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        file_path = input.get("file_path")
        if not file_path:
            return ToolResult.error("file_path is required")

        full_path = os.path.join(context.work_dir, file_path)

        try:
            async with aiofiles.open(full_path, "r", encoding="utf-8") as f:
                content = await f.read()
            return ToolResult.success(content)
        except Exception as e:
            return ToolResult.error(f"Failed to read file: {e}")
```

### 3.2 WriteFileTool

```python
import aiofiles
import os

class WriteFileTool:
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
        full_path = os.path.join(context.work_dir, file_path)

        try:
            os.makedirs(os.path.dirname(full_path), exist_ok=True)
            async with aiofiles.open(full_path, "w", encoding="utf-8") as f:
                await f.write(content)
            return ToolResult.success(f"File written: {full_path}")
        except Exception as e:
            return ToolResult.error(f"Failed to write file: {e}")
```

### 3.3 UpdateFileTool

```python
import aiofiles
import os

class UpdateFileTool:
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
                "old_string": {"type": "string"},
                "new_string": {"type": "string"},
                "replace_all": {"type": "boolean", "default": False},
            },
            "required": ["file_path", "old_string", "new_string"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        file_path = input.get("file_path", "")
        old_str = input.get("old_string", "")
        new_str = input.get("new_string", "")
        replace_all = input.get("replace_all", False)
        full_path = os.path.join(context.work_dir, file_path)

        try:
            async with aiofiles.open(full_path, "r", encoding="utf-8") as f:
                content = await f.read()

            if replace_all:
                new_content = content.replace(old_str, new_str)
            else:
                new_content = content.replace(old_str, new_str, 1)

            if new_content == content:
                return ToolResult.error("old_string not found in file")

            async with aiofiles.open(full_path, "w", encoding="utf-8") as f:
                await f.write(new_content)

            return ToolResult.success(f"File updated: {full_path}")
        except Exception as e:
            return ToolResult.error(f"Failed to update file: {e}")
```

### 3.4 BashTool

```python
import asyncio

class BashTool:
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
                "command": {"type": "string", "description": "The shell command"},
                "description": {"type": "string"},
                "timeout": {"type": "integer", "default": 120000},
            },
            "required": ["command"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        command = input.get("command", "")
        timeout_ms = input.get("timeout", 120_000) / 1000  # 转换为秒

        try:
            proc = await asyncio.create_subprocess_shell(
                command,
                cwd=context.work_dir,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )

            try:
                stdout, stderr = await asyncio.wait_for(
                    proc.communicate(), timeout=timeout_ms
                )
            except asyncio.TimeoutError:
                proc.kill()
                return ToolResult.error(f"Command timed out after {timeout_ms}s")

            output = stdout.decode("utf-8", errors="replace")
            if stderr:
                output += f"\n[stderr]\n{stderr.decode('utf-8', errors='replace')}"

            return ToolResult(content=output, is_error=proc.returncode != 0)
        except Exception as e:
            return ToolResult.error(f"Failed to execute: {e}")
```

### 3.5 CurlTool

```python
import httpx

class CurlTool:
    def __init__(self):
        self._client = httpx.AsyncClient()

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
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"]},
                "headers": {"type": "object"},
                "body": {"type": "string"},
                "timeout": {"type": "integer", "default": 30000},
            },
            "required": ["url"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        url = input.get("url", "")
        method = input.get("method", "GET")
        body = input.get("body")
        headers = input.get("headers", {})
        timeout = input.get("timeout", 30_000) / 1000

        try:
            response = await self._client.request(
                method=method,
                url=url,
                headers=headers,
                content=body,
                timeout=httpx.Timeout(timeout),
            )
            return ToolResult.success(response.text)
        except Exception as e:
            return ToolResult.error(f"HTTP error: {e}")
```

## 4. API 调用层

### 4.1 AnthropicClient

```python
import httpx
import json
from typing import AsyncIterator

class AnthropicClient:
    def __init__(self, config: AgentConfig):
        self._config = config
        self._client = httpx.AsyncClient(
            base_url=config.base_url,
            headers={
                "x-api-key": config.api_key,
                "anthropic-version": "2023-06-01",
                "Content-Type": "application/json",
            },
            timeout=config.timeout_ms / 1000,
        )

    async def stream_messages(
        self,
        messages: list[Message],
        system: str,
        tools: list[dict],
    ) -> AsyncIterator[dict]:
        """发送流式请求，返回 SSE 事件"""
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

    def _message_to_dict(self, message: Message) -> dict:
        return {
            "role": message.role.value,
            "content": [self._block_to_dict(b) for b in message.content],
        }

    def _block_to_dict(self, block: ContentBlock) -> dict:
        if isinstance(block, TextBlock):
            return {"type": "text", "text": block.text}
        elif isinstance(block, ToolUseBlock):
            return {"type": "tool_use", "name": block.name, "id": block.id, "input": block.input}
        elif isinstance(block, ToolResultBlock):
            result = {"type": "tool_result", "tool_use_id": block.tool_use_id, "content": block.content}
            if block.is_error is not None:
                result["is_error"] = block.is_error
            return result
        elif isinstance(block, ThinkingBlock):
            return {"type": "thinking", "thinking": block.thinking}
        else:
            raise ValueError(f"Unknown block type: {type(block)}")
```

## 5. Agent Loop 实现

```python
import asyncio

class Agent:
    # ... constructor from above

    async def _run_loop(self) -> AsyncIterator[Event]:
        from .prompt import build_system_prompt

        system_prompt = build_system_prompt(self._tools, self._config.system_prompt)

        while self._turn_count < self._config.max_turns:
            self._turn_count += 1
            yield Event.turn_start(self._turn_count)

            # 构建工具定义
            tool_defs = [
                {
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                }
                for t in self._tools.values()
            ]

            client = AnthropicClient(self._config)

            # 流式接收
            assistant_content: list[ContentBlock] = []
            yield Event(EventType.MESSAGE_START, {})

            async for event in client.stream_messages(
                self._message_history,
                system_prompt,
                tool_defs,
            ):
                event_type = event.get("type")

                if event_type == "content_block_start":
                    block = self._parse_block(event["content_block"])
                    assistant_content.append(block)

                elif event_type == "content_block_delta":
                    delta = event["delta"]
                    if delta.get("type") == "text_delta":
                        yield Event.message_delta(delta["text"])
                    elif delta.get("type") == "thinking_delta":
                        yield Event.thinking_delta(delta["thinking"])

                elif event_type == "message_stop":
                    yield Event(EventType.MESSAGE_END, {})

            # 将 assistant 消息加入历史
            self._message_history.append(Message.assistant(assistant_content))

            # 提取 tool_use
            tool_uses = [b for b in assistant_content if isinstance(b, ToolUseBlock)]

            # 若无 tool_use，任务完成
            if not tool_uses:
                final_text = self._extract_text(assistant_content)
                yield Event.complete(final_text)
                return

            # 执行工具
            results = await self._execute_tools(tool_uses)

            # 将 tool_result 加入历史
            self._message_history.append(Message(
                role=Role.USER,
                content=results,
            ))

        yield Event.error("Max turns reached")

    async def _execute_tools(self, tool_uses: list[ToolUseBlock]) -> list[ContentBlock]:
        # 分组
        read_only: list[ToolUseBlock] = []
        write: list[ToolUseBlock] = []

        for block in tool_uses:
            tool = self._tools.get(block.name)
            if tool and tool.is_read_only:
                read_only.append(block)
            else:
                write.append(block)

        results: list[ContentBlock] = []

        # 并发执行只读工具
        if read_only:
            read_tasks = [self._execute_single_tool(b) for b in read_only]
            read_results = await asyncio.gather(*read_tasks, return_exceptions=True)
            for r in read_results:
                if isinstance(r, Exception):
                    results.append(ToolResultBlock(
                        tool_use_id="unknown",
                        content=str(r),
                        is_error=True,
                    ))
                else:
                    results.append(r)

        # 串行执行写工具
        for block in write:
            result = await self._execute_single_tool(block)
            results.append(result)

        return results

    async def _execute_single_tool(self, block: ToolUseBlock) -> ContentBlock:
        tool = self._tools.get(block.name)
        if not tool:
            return ToolResultBlock(
                tool_use_id=block.id,
                content=f"Tool not found: {block.name}",
                is_error=True,
            )

        context = ToolContext(
            work_dir=self._config.work_dir,
            message_history=list(self._message_history),
        )

        try:
            result = await tool.call(block.input, context)
        except Exception as e:
            result = ToolResult.error(str(e))

        return ToolResultBlock(
            tool_use_id=block.id,
            content=result.content,
            is_error=result.is_error,
        )

    def _extract_text(self, blocks: list[ContentBlock]) -> str:
        return "".join(b.text for b in blocks if isinstance(b, TextBlock))

    def _parse_block(self, data: dict) -> ContentBlock:
        block_type = data.get("type")
        if block_type == "text":
            return TextBlock(text=data["text"])
        elif block_type == "tool_use":
            return ToolUseBlock(
                name=data["name"],
                id=data["id"],
                input=data.get("input", {}),
            )
        elif block_type == "thinking":
            return ThinkingBlock(thinking=data["thinking"])
        else:
            raise ValueError(f"Unknown block type: {block_type}")
```

## 6. 自定义工具注册

```python
# 定义自定义工具
class MyTool:
    @property
    def name(self) -> str:
        return "my_tool"

    @property
    def description(self) -> str:
        return "Does something custom"

    @property
    def is_read_only(self) -> bool:
        return True

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "param1": {"type": "string"},
            },
            "required": ["param1"],
        }

    async def call(self, input: dict, context: ToolContext) -> ToolResult:
        param1 = input.get("param1", "")
        return ToolResult.success(f"Processed: {param1}")


# 使用
from agentlib import Agent, AgentConfig

async def main():
    config = AgentConfig(
        api_key="sk-...",
        model="claude-sonnet-4-6",
        tools=[MyTool()],
    )

    agent = Agent(config)
    async for event in agent.run("Hello"):
        print(event)

asyncio.run(main())
```

## 7. 目录结构

```
python/
├── pyproject.toml
├── src/
│   └── agentlib/
│       ├── __init__.py       # 模块导出
│       ├── agent.py          # Agent 类
│       ├── config.py         # AgentConfig
│       ├── types.py          # Message, ContentBlock, Role, Event
│       ├── tool.py           # Tool Protocol, ToolContext, ToolResult
│       ├── tools/
│       │   ├── __init__.py
│       │   ├── read_file.py
│       │   ├── write_file.py
│       │   ├── update_file.py
│       │   ├── bash.py
│       │   └── curl.py
│       ├── client.py         # Anthropic API 客户端
│       ├── stream.py         # SSE 流式解析
│       └── prompt.py         # 系统提示词构建
└── tests/
    ├── test_agent.py
    ├── test_tools.py
    └── fixtures/
```

## 8. 依赖 (pyproject.toml)

```toml
[project]
name = "agentlib"
version = "0.1.0"
description = "A lightweight agent library for Anthropic API"
requires-python = ">=3.10"
dependencies = [
    "httpx>=0.27.0",
    "aiofiles>=23.0.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0.0",
    "pytest-asyncio>=0.23.0",
    "pytest-httpx>=0.30.0",
    "respx>=0.21.0",
]
```
