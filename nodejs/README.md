# @zhangzichao2008/agent-lib

一个轻量级 Node.js Agent 库，核心能力是对接 Anthropic Messages API（以及兼容 API），实现标准的 agent-loop：模型可自主发起工具调用（tool_use），库负责执行工具并把结果（tool_result）回填给模型，直到产出最终回答。

## 安装

```bash
npm i @zhangzichao2008/agent-lib
```

需要 Node.js >= 18。

## 环境变量

- 必填：`ANTHROPIC_AUTH_TOKEN` 或 `ANTHROPIC_API_KEY`
- 可选：`ANTHROPIC_BASE_URL`（默认 `https://api.anthropic.com`）
- 可选：`ANTHROPIC_MODEL`（默认 `claude-sonnet-4-6`）
- 可选：`AGENT_WORK_DIR`（工作目录，文件/命令工具的沙箱基准目录）

## 基本使用

```ts
import { Agent, resolveConfig } from '@zhangzichao2008/agent-lib';

async function main() {
  const config = resolveConfig({
    apiKey: process.env.ANTHROPIC_AUTH_TOKEN || process.env.ANTHROPIC_API_KEY || '',
    baseUrl: process.env.ANTHROPIC_BASE_URL,
    model: process.env.ANTHROPIC_MODEL,
  });

  const agent = new Agent(config);

  for await (const event of agent.run('Please list the files in the current directory')) {
    if (event.type === 'message_delta') {
      process.stdout.write(event.data.text as string);
    } else if (event.type === 'complete') {
      console.log('\n\n[Complete]', event.data.final_content);
    } else if (event.type === 'error') {
      console.error('\n[Error]', event.data.message);
    }
  }
}

main().catch(console.error);
```

## 事件类型

`agent.run()` 返回一个 `AsyncIterable<Event>`，包含以下事件：

| 事件类型 | 说明 |
|---------|------|
| `message_delta` | 流式文本增量，`event.data.text` |
| `message_stop` | 模型回复结束 |
| `tool_use` | 模型发起工具调用，`event.data.name` / `event.data.input` |
| `tool_result` | 工具执行完成，`event.data.result` |
| `complete` | Agent 循环结束，`event.data.final_content` |
| `error` | 发生错误，`event.data.message` |

## 内置工具

- `read_file` — 读取文件
- `write_file` — 写入/创建文件
- `update_file` — 替换文件内容
- `bash` — 执行 shell 命令
- `curl` — HTTP 请求
- `subagent` — 子 Agent 调用

## API

### `resolveConfig(options?)`

创建 `AgentConfig` 配置对象。支持从环境变量和参数中合并配置。

### `new Agent(config)`

```ts
interface AgentConfig {
  apiKey: string;
  baseUrl?: string;
  model?: string;
  maxTokens?: number;
  tools?: Tool[];
  workDir?: string;
}
```

### `agent.run(prompt)`

执行 agent-loop，返回 `AsyncIterable<Event>`。

## 许可证

Proprietary
