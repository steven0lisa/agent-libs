# PRD - NodeJS (TypeScript) 实现

## 1. 语言特性映射

| 通用概念 | TypeScript 实现 |
|----------|-----------------|
| Agent | `class Agent` |
| Tool | `interface ITool` |
| Message | `interface Message` |
| ContentBlock | `type ContentBlock = ...` (union) |
| Event | `type Event = ...` (union/discriminated) |
| Config | `interface AgentConfig` |
| 异步 | `async`/`await` + `Promise` |
| 事件流 | `AsyncGenerator<Event>` 或 `EventEmitter` |
| 错误处理 | `throw`/`catch` + 自定义 Error |
| JSON | `Record<string, unknown>` / JSON Schema types |
| HTTP | `fetch` (Node 18+) |

## 2. 核心类型设计

### 2.1 ContentBlock

```typescript
export type ContentBlock =
  | { type: 'text'; text: string }
  | { type: 'tool_use'; name: string; id: string; input: Record<string, unknown> }
  | { type: 'tool_result'; tool_use_id: string; content: string; is_error?: boolean }
  | { type: 'thinking'; thinking: string; signature?: string };

// 类型守卫
export function isToolUse(block: ContentBlock): block is Extract<ContentBlock, { type: 'tool_use' }> {
  return block.type === 'tool_use';
}

export function isText(block: ContentBlock): block is Extract<ContentBlock, { type: 'text' }> {
  return block.type === 'text';
}
```

### 2.2 Message

```typescript
export type Role = 'user' | 'assistant';

export interface Message {
  role: Role;
  content: ContentBlock[];
}

export function userMessage(text: string): Message {
  return { role: 'user', content: [{ type: 'text', text }] };
}
```

### 2.3 Tool Interface

```typescript
export interface ITool {
  readonly name: string;
  readonly description: string;
  readonly inputSchema: Record<string, unknown>;

  /** 是否只读操作，影响并发策略 */
  readonly isReadOnly?: boolean;

  call(input: Record<string, unknown>, context: ToolContext): Promise<ToolResult>;
}

export interface ToolContext {
  workDir: string;
  messageHistory: Message[];
}

export interface ToolResult {
  content: string;
  isError: boolean;
}

// 便捷函数
export function success(content: string): ToolResult {
  return { content, isError: false };
}

export function error(content: string): ToolResult {
  return { content, isError: true };
}
```

### 2.4 Event

```typescript
export type Event =
  | { type: 'turn_start'; turn: number }
  | { type: 'message_start' }
  | { type: 'message_delta'; text: string }
  | { type: 'thinking_delta'; thinking: string }
  | { type: 'message_end' }
  | { type: 'tool_use_start'; name: string; id: string; input: Record<string, unknown> }
  | { type: 'tool_use_end'; name: string; id: string; result: ToolResult }
  | { type: 'error'; message: string }
  | { type: 'complete'; finalContent: string };
```

### 2.5 AgentConfig

```typescript
export interface AgentConfig {
  /** API 基础地址，默认: https://api.anthropic.com */
  baseUrl?: string;
  /** API 密钥 */
  apiKey: string;
  /** 模型名称，默认: claude-sonnet-4-6 */
  model?: string;
  /** 工作目录，默认: process.cwd() */
  workDir?: string;
  /** 单次请求最大输出 token，默认: 8192 */
  maxTokens?: number;
  /** 最大对话轮数，默认: 100 */
  maxTurns?: number;
  /** 追加的系统提示词 */
  systemPrompt?: string;
  /** 请求超时(ms)，默认: 120000 */
  timeoutMs?: number;
  /** 是否流式，默认: true */
  stream?: boolean;
  /** 初始自定义工具 */
  tools?: ITool[];
}

export const DEFAULT_CONFIG: Required<Pick<AgentConfig, 'baseUrl' | 'model' | 'maxTokens' | 'maxTurns' | 'timeoutMs' | 'stream'>> = {
  baseUrl: 'https://api.anthropic.com',
  model: 'claude-sonnet-4-6',
  maxTokens: 8192,
  maxTurns: 100,
  timeoutMs: 120_000,
  stream: true,
};
```

### 2.6 Agent

```typescript
export class Agent {
  private config: Required<AgentConfig>;
  private tools: Map<string, ITool>;
  private messageHistory: Message[];
  private turnCount: number;

  constructor(config: AgentConfig) {
    this.config = {
      ...DEFAULT_CONFIG,
      workDir: process.cwd(),
      ...config,
    };
    this.tools = new Map();
    this.messageHistory = [];
    this.turnCount = 0;

    // 注册预定义工具
    this.registerTool(new ReadFileTool());
    this.registerTool(new WriteFileTool());
    this.registerTool(new UpdateFileTool());
    this.registerTool(new BashTool());
    this.registerTool(new CurlTool());

    // 注册自定义工具
    config.tools?.forEach(t => this.registerTool(t));
  }

  registerTool(tool: ITool): void {
    this.tools.set(tool.name, tool);
  }

  unregisterTool(name: string): void {
    this.tools.delete(name);
  }

  listTools(): ITool[] {
    return Array.from(this.tools.values());
  }

  /** 核心方法：运行 agent loop，返回异步事件生成器 */
  async *run(input: string): AsyncGenerator<Event> {
    this.messageHistory.push(userMessage(input));
    yield* this.runLoop();
  }

  /** 以已有消息历史继续对话 */
  async *chat(messages: Message[]): AsyncGenerator<Event> {
    this.messageHistory.push(...messages);
    yield* this.runLoop();
  }

  getMessageHistory(): readonly Message[] {
    return this.messageHistory;
  }

  clearHistory(): void {
    this.messageHistory = [];
    this.turnCount = 0;
  }
}
```

## 3. 预定义工具实现

### 3.1 ReadFileTool

```typescript
export class ReadFileTool implements ITool {
  readonly name = 'read_file';
  readonly description = 'Read file contents. Supports text, images, PDFs, notebooks.';
  readonly isReadOnly = true;

  readonly inputSchema = {
    type: 'object',
    properties: {
      file_path: { type: 'string', description: 'Absolute or relative path to the file' },
      offset: { type: 'integer', description: 'Line number to start reading from' },
      limit: { type: 'integer', description: 'Maximum number of lines to read' },
    },
    required: ['file_path'],
  };

  async call(input: Record<string, unknown>, context: ToolContext): Promise<ToolResult> {
    const filePath = input.file_path as string;
    const fullPath = path.resolve(context.workDir, filePath);

    try {
      const content = await fs.promises.readFile(fullPath, 'utf-8');
      return success(content);
    } catch (err) {
      return error(`Failed to read file: ${(err as Error).message}`);
    }
  }
}
```

### 3.2 WriteFileTool

```typescript
export class WriteFileTool implements ITool {
  readonly name = 'write_file';
  readonly description = 'Write content to a file. Creates if not exists, overwrites if exists.';
  readonly isReadOnly = false;

  readonly inputSchema = {
    type: 'object',
    properties: {
      file_path: { type: 'string' },
      content: { type: 'string' },
    },
    required: ['file_path', 'content'],
  };

  async call(input: Record<string, unknown>, context: ToolContext): Promise<ToolResult> {
    const filePath = input.file_path as string;
    const content = input.content as string;
    const fullPath = path.resolve(context.workDir, filePath);

    try {
      await fs.promises.mkdir(path.dirname(fullPath), { recursive: true });
      await fs.promises.writeFile(fullPath, content, 'utf-8');
      return success(`File written successfully: ${fullPath}`);
    } catch (err) {
      return error(`Failed to write file: ${(err as Error).message}`);
    }
  }
}
```

### 3.3 UpdateFileTool

```typescript
export class UpdateFileTool implements ITool {
  readonly name = 'update_file';
  readonly description = 'Update a file by replacing old_string with new_string.';
  readonly isReadOnly = false;

  readonly inputSchema = {
    type: 'object',
    properties: {
      file_path: { type: 'string' },
      old_string: { type: 'string', description: 'The text to replace' },
      new_string: { type: 'string', description: 'The replacement text' },
      replace_all: { type: 'boolean', default: false, description: 'Replace all occurrences' },
    },
    required: ['file_path', 'old_string', 'new_string'],
  };

  async call(input: Record<string, unknown>, context: ToolContext): Promise<ToolResult> {
    const filePath = input.file_path as string;
    const oldStr = input.old_string as string;
    const newStr = input.new_string as string;
    const replaceAll = (input.replace_all as boolean) ?? false;
    const fullPath = path.resolve(context.workDir, filePath);

    try {
      let content = await fs.promises.readFile(fullPath, 'utf-8');
      const original = content;

      content = replaceAll
        ? content.split(oldStr).join(newStr)
        : content.replace(oldStr, newStr);

      if (content === original) {
        return error('old_string not found in file');
      }

      await fs.promises.writeFile(fullPath, content, 'utf-8');
      return success(`File updated successfully: ${fullPath}`);
    } catch (err) {
      return error(`Failed to update file: ${(err as Error).message}`);
    }
  }
}
```

### 3.4 BashTool

```typescript
export class BashTool implements ITool {
  readonly name = 'bash';
  readonly description = 'Execute a shell command in the working directory.';

  readonly inputSchema = {
    type: 'object',
    properties: {
      command: { type: 'string', description: 'The shell command to execute' },
      description: { type: 'string', description: 'A brief description of what the command does' },
      timeout: { type: 'integer', description: 'Timeout in milliseconds', default: 120000 },
    },
    required: ['command'],
  };

  async call(input: Record<string, unknown>, context: ToolContext): Promise<ToolResult> {
    const command = input.command as string;
    const timeoutMs = (input.timeout as number) ?? 120_000;

    return new Promise((resolve) => {
      const child = spawn('sh', ['-c', command], {
        cwd: context.workDir,
        shell: false,
      });

      let stdout = '';
      let stderr = '';
      let timedOut = false;

      const timer = setTimeout(() => {
        timedOut = true;
        child.kill('SIGKILL');
      }, timeoutMs);

      child.stdout?.on('data', (data) => { stdout += data.toString(); });
      child.stderr?.on('data', (data) => { stderr += data.toString(); });

      child.on('close', (code) => {
        clearTimeout(timer);
        if (timedOut) {
          resolve(error(`Command timed out after ${timeoutMs}ms`));
          return;
        }

        const output = stderr ? `${stdout}\n[stderr]\n${stderr}` : stdout;
        resolve({ content: output, isError: code !== 0 });
      });

      child.on('error', (err) => {
        clearTimeout(timer);
        resolve(error(`Failed to execute: ${err.message}`));
      });
    });
  }
}
```

### 3.5 CurlTool

```typescript
export class CurlTool implements ITool {
  readonly name = 'curl';
  readonly description = 'Make an HTTP request.';
  readonly isReadOnly = true;

  readonly inputSchema = {
    type: 'object',
    properties: {
      url: { type: 'string' },
      method: { type: 'string', enum: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'], default: 'GET' },
      headers: { type: 'object', additionalProperties: { type: 'string' } },
      body: { type: 'string' },
      timeout: { type: 'integer', default: 30000 },
    },
    required: ['url'],
  };

  async call(input: Record<string, unknown>, _context: ToolContext): Promise<ToolResult> {
    const url = input.url as string;
    const method = (input.method as string) ?? 'GET';
    const body = input.body as string | undefined;
    const headers = input.headers as Record<string, string> | undefined;
    const timeout = (input.timeout as number) ?? 30_000;

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeout);

    try {
      const response = await fetch(url, {
        method,
        headers,
        body,
        signal: controller.signal,
      });

      const text = await response.text();
      return success(text);
    } catch (err) {
      return error(`HTTP error: ${(err as Error).message}`);
    } finally {
      clearTimeout(timer);
    }
  }
}
```

## 4. API 调用层

### 4.1 AnthropicClient

```typescript
export interface ToolDefinition {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
}

export interface APIRequest {
  model: string;
  messages: Message[];
  system?: string;
  tools?: ToolDefinition[];
  max_tokens: number;
  stream?: boolean;
}

export class AnthropicClient {
  private config: Required<AgentConfig>;

  constructor(config: Required<AgentConfig>) {
    this.config = config;
  }

  async *streamMessages(
    messages: Message[],
    system: string,
    tools: ToolDefinition[],
  ): AsyncGenerator<StreamEvent> {
    const request: APIRequest = {
      model: this.config.model,
      messages,
      system,
      tools,
      max_tokens: this.config.maxTokens,
      stream: true,
    };

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.config.timeoutMs);

    try {
      const response = await fetch(`${this.config.baseUrl}/v1/messages`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': this.config.apiKey,
          'anthropic-version': '2023-06-01',
          'Accept': 'text/event-stream',
        },
        body: JSON.stringify(request),
        signal: controller.signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`);
      }

      const reader = response.body?.getReader();
      if (!reader) throw new Error('No response body');

      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() ?? '';

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') return;

            try {
              yield JSON.parse(data) as StreamEvent;
            } catch {
              // skip malformed lines
            }
          }
        }
      }
    } finally {
      clearTimeout(timer);
    }
  }
}
```

## 5. Agent Loop 实现

```typescript
export class Agent {
  // ... constructor and other methods from above

  private async *runLoop(): AsyncGenerator<Event> {
    const systemPrompt = buildSystemPrompt(this.tools, this.config.systemPrompt);

    while (this.turnCount < this.config.maxTurns) {
      this.turnCount++;
      yield { type: 'turn_start', turn: this.turnCount };

      // 构建工具定义
      const toolDefs: ToolDefinition[] = Array.from(this.tools.values()).map(t => ({
        name: t.name,
        description: t.description,
        input_schema: t.inputSchema,
      }));

      const client = new AnthropicClient(this.config);

      // 流式接收
      let assistantContent: ContentBlock[] = [];
      let toolUseBlocks: ContentBlock[] = [];

      yield { type: 'message_start' };

      for await (const event of client.streamMessages(
        this.messageHistory,
        systemPrompt,
        toolDefs,
      )) {
        // 解析流事件
        switch (event.type) {
          case 'content_block_start': {
            const block = event.content_block;
            assistantContent.push(block);
            break;
          }
          case 'content_block_delta': {
            const delta = event.delta;
            if (delta.type === 'text_delta') {
              yield { type: 'message_delta', text: delta.text };
            } else if (delta.type === 'thinking_delta') {
              yield { type: 'thinking_delta', thinking: delta.thinking };
            }
            break;
          }
          case 'message_stop': {
            yield { type: 'message_end' };
            break;
          }
        }
      }

      // 提取 tool_use blocks
      toolUseBlocks = assistantContent.filter(isToolUse);

      // 将 assistant 消息加入历史
      this.messageHistory.push({ role: 'assistant', content: assistantContent });

      // 若无 tool_use，任务完成
      if (toolUseBlocks.length === 0) {
        const finalText = extractText(assistantContent);
        yield { type: 'complete', finalContent: finalText };
        return;
      }

      // 执行工具
      const results = await this.executeTools(toolUseBlocks);

      // 将 tool_result 加入历史
      this.messageHistory.push({ role: 'user', content: results });
    }

    yield { type: 'error', message: 'Max turns reached' };
  }

  private async executeTools(toolUses: ContentBlock[]): Promise<ContentBlock[]> {
    // 分组
    const readOnly: Extract<ContentBlock, { type: 'tool_use' }>[] = [];
    const write: Extract<ContentBlock, { type: 'tool_use' }>[] = [];

    for (const block of toolUses) {
      if (!isToolUse(block)) continue;
      const tool = this.tools.get(block.name);
      if (tool?.isReadOnly) {
        readOnly.push(block);
      } else {
        write.push(block);
      }
    }

    const results: ContentBlock[] = [];

    // 并发执行只读工具
    const readResults = await Promise.all(
      readOnly.map(block => this.executeSingleTool(block)),
    );
    results.push(...readResults);

    // 串行执行写工具
    for (const block of write) {
      results.push(await this.executeSingleTool(block));
    }

    return results;
  }

  private async executeSingleTool(
    block: Extract<ContentBlock, { type: 'tool_use' }>,
  ): Promise<ContentBlock> {
    const tool = this.tools.get(block.name);
    if (!tool) {
      return {
        type: 'tool_result',
        tool_use_id: block.id,
        content: `Tool not found: ${block.name}`,
        is_error: true,
      };
    }

    const context: ToolContext = {
      workDir: this.config.workDir,
      messageHistory: this.messageHistory,
    };

    const result = await tool.call(block.input, context);

    return {
      type: 'tool_result',
      tool_use_id: block.id,
      content: result.content,
      is_error: result.isError,
    };
  }
}
```

## 6. 自定义工具注册

```typescript
// 定义自定义工具
const myTool: ITool = {
  name: 'my_tool',
  description: 'Does something custom',
  isReadOnly: true,
  inputSchema: {
    type: 'object',
    properties: {
      param1: { type: 'string' },
    },
    required: ['param1'],
  },
  async call(input, context) {
    const param1 = input.param1 as string;
    return success(`Processed: ${param1}`);
  },
};

// 使用
const agent = new Agent({
  apiKey: 'sk-...',
  model: 'claude-sonnet-4-6',
  tools: [myTool],
});

for await (const event of agent.run('Hello')) {
  console.log(event);
}
```

## 7. 目录结构

```
nodejs/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts           # 模块导出
│   ├── agent.ts           # Agent 类
│   ├── config.ts          # AgentConfig, DEFAULT_CONFIG
│   ├── types.ts           # Message, ContentBlock, Role, Event
│   ├── tool.ts            # ITool, ToolContext, ToolResult
│   ├── tools/
│   │   ├── index.ts       # 工具导出
│   │   ├── read-file.ts
│   │   ├── write-file.ts
│   │   ├── update-file.ts
│   │   ├── bash.ts
│   │   └── curl.ts
│   ├── client.ts          # Anthropic API 客户端
│   ├── stream.ts          # SSE 流式解析
│   └── prompt.ts          # 系统提示词构建
├── tests/
│   ├── agent.test.ts
│   ├── tools.test.ts
│   └── fixtures/
└── dist/                  # 编译输出
```

## 8. 依赖 (package.json)

```json
{
  "name": "@zhangzichao2008/agent-lib",
  "version": "0.1.0",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "node --test dist/**/*.test.js"
  },
  "devDependencies": {
    "typescript": "^5.4.0",
    "@types/node": "^20.0.0"
  },
  "engines": {
    "node": ">=18.0.0"
  }
}
```

**无生产环境依赖**。Node 18+ 内置 `fetch` 和 `ReadableStream` 支持，无需额外 HTTP 库。

## 9. Skills 系统实现

### 9.1 概述

Skills 系统允许 Agent 从 `~/.claude/skills/` 目录发现并调用 SKILL.md 格式的技能文件。系统设计参考 Claude Code 的 skill 机制，但采用更简化的 Library 层级实现——不涉及 CLI 用户输入前缀匹配（`/skillname`），仅提供模型驱动的 `skill` Tool 调用。

默认关闭，需通过 `enableSkills: true` 配置开启。

### 9.2 SKILL.md 格式规范

每个技能存放在 `~/.claude/skills/<skill-name>/SKILL.md` 目录下，包含 YAML frontmatter 元数据和 Markdown 内容体：

```markdown
---
name: my-skill
description: 技能描述，说明何时使用
when_to_use: 更详细的适用场景说明
allowed_tools: bash, read_file
model: claude-sonnet-4-6
context: inline       # inline | fork，默认 inline
version: 1.0.0
---

# 技能标题

## 工作流程

### 步骤 1: ...
```

**Frontmatter 字段：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 技能名称，用于 skill tool 查找 |
| `description` | string | 否 | 简短描述，注入 system prompt 的 skill_listing |
| `when_to_use` | string | 否 | 更详细的适用场景 |
| `allowed_tools` | array | 否 | 技能可用的工具列表 |
| `model` | string | 否 | 指定使用的模型（fork 模式下） |
| `context` | string | 否 | 执行模式：`inline`（默认，返回内容给主模型）或 `fork`（创建子 Agent 独立执行） |
| `version` | string | 否 | 技能版本号 |

### 9.3 技能发现机制（SkillLoader）

`SkillLoader` 负责从磁盘发现和缓存 SKILL.md 文件：

**扫描路径：**
1. **用户目录**：`~/.claude/skills/<name>/SKILL.md`（默认，优先级高）
2. **项目目录**（可选）：`<projectDir>/.claude/skills/<name>/SKILL.md`

**行为：**
- 构造函数同步，不执行 I/O
- `discoverAll()` 第一次调用时扫描磁盘，结果缓存
- `findByName(name)` 精确匹配技能名称
- `clearCache()` 清空缓存
- 同名技能：用户目录技能优先于项目目录技能
- 无效技能（无 SKILL.md、元数据缺失、解析错误）静默跳过

### 9.4 YAML Frontmatter 解析

`parseFrontmatter()` 实现简易 YAML 前置元数据解析，无外部依赖：

- 正则表达式提取 `---` 块
- 逐行解析 `key: value` 格式
- 支持的值类型：
  - **字符串**：自动去除两侧引号
  - **布尔值**：`true` / `false`
  - **数字**：自动类型转换
  - **数组**：`- item1, item2` 格式，按逗号分割

```typescript
// 示例输出
{
  name: "my-skill",
  description: "技能描述",
  when_to_use: "适用场景说明",
  allowed_tools: ["bash", "read_file"],
  context: "inline"
}
```

### 9.5 变量替换规则

技能内容中的变量在执行时动态替换：

| 变量 | 替换为 |
|------|--------|
| `$ARGUMENTS` | `skill` tool 调用时传入的 `args` 参数值 |
| `${CLAUDE_SKILL_DIR}` | 技能的目录路径，用于引用同目录下的资源文件 |
| `${ENV:VAR_NAME}` | 环境变量 `VAR_NAME` 的值，不存在则为空字符串 |

替换由纯函数 `substituteVariables()` 执行，不产生副作用。

### 9.6 SkillTool 实现

`SkillTool` 实现了 `ITool` 接口，作为模型调用技能的入口：

```typescript
class SkillTool implements ITool {
  name = 'skill';
  description = 'Load a skill and get its instructions...';
  isReadOnly = true;

  inputSchema = {
    type: 'object',
    properties: {
      skill: { type: 'string', description: '技能名称' },
      args: { type: 'string', description: '传递给技能的参数，通过 $ARGUMENTS 引用' },
    },
    required: ['skill'],
  };
}
```

**执行流程（`call` 方法）：**

1. 提取 `input.skill` 和 `input.args`
2. 通过 `SkillLoader.findByName()` 查找技能
3. 技能不存在时返回错误（含可用技能列表）
4. 执行变量替换（args → `$ARGUMENTS`, skillDir → `${CLAUDE_SKILL_DIR}`, env vars → `${ENV:...}`）
5. 构建返回结果：包含技能名、描述、处理后的内容

### 9.7 内联 vs Fork 模式

两种执行模式通过 SKILL.md 的 `context` 字段控制：

**内联模式（默认）**：`context: inline`
- Skill 内容通过 `tool_result` 返回给主模型
- 主模型根据技能内容自主操作
- 适用于：指令型技能、参考文档、代码规范等

**Fork 模式**：`context: fork`
- 创建子 Agent（SubAgent），以技能内容作为 systemPrompt 独立执行
- 子 Agent 不继承 skill 工具自身（避免循环引用）
- 独立 tool budget，结果返回给主模型
- 适用于：需要独立执行工作流的复杂技能（如浏览器自动化、UI 审查）

### 9.8 System Prompt 注入

当启用 Skills 且有可用技能时，`buildSystemPrompt()` 会在 system prompt 末尾注入 `## Available Skills` 列表：

```
## Available Skills

### git-commit
Description: 生成规范的 Git 提交信息...

### browser-automation
Description: Vision-driven browser automation...
When to use: 当需要浏览器自动化、网页测试时使用

To use a skill, call the `skill` tool with the skill name and optional arguments.
```

注入条件：
- `skills` 参数非空且长度 > 0
- 只注入基本元数据（name, description, when_to_use），不注入完整内容
- 提示模型使用 `skill` tool 加载完整内容

### 9.9 Agent 配置集成

在 `AgentConfig` 中新增以下字段：

```typescript
export interface AgentConfig {
  // ... 现有字段 ...

  /** 启用技能系统，默认 false */
  enableSkills?: boolean;
  /** 自定义技能目录，默认 ~/.claude/skills */
  skillsDir?: string;
  /** 是否同时扫描项目级技能 (.claude/skills) */
  includeProjectSkills?: boolean;
  /** 项目根目录，用于项目级技能发现 */
  skillsProjectDir?: string;
}
```

配置示例：

```typescript
const agent = new Agent({
  apiKey: 'sk-...',
  enableSkills: true,                          // 启用技能
  skillsDir: '/custom/path/skills',            // 自定义技能目录
  includeProjectSkills: true,                  // 包含项目级技能
  skillsProjectDir: '/path/to/project',        // 项目根目录
});
```

### 9.10 自定义技能示例

在 `~/.claude/skills/my-helper/SKILL.md`：

```markdown
---
name: my-helper
description: 自定义辅助技能
when_to_use: 当需要处理特定任务时使用
allowed_tools: bash, read_file
---

# My Helper Skill

## 工作流程

1. 读取配置文件：`read_file ${CLAUDE_SKILL_DIR}/config.json`
2. 根据用户参数 `$ARGUMENTS` 执行操作
3. 输出结果
```

### 9.11 Agent 生命周期集成

```
Agent 构造函数
  └─ initTools()
       └─ enableSkills === true
            ├─ 创建 SkillLoader (同步，不执行 I/O)
            └─ 注册 SkillTool

第一次 runLoop()
  └─ 懒加载：SkillLoader.discoverAll() (I/O，缓存结果)
  └─ 传入 buildSystemPrompt(skills)
       └─ 注入 ## Available Skills 列表

模型调用 `skill` tool
  └─ SkillTool.call()
       ├─ SkillLoader.findByName() (命中缓存)
       ├─ substituteVariables() (变量替换)
       └─ 返回处理后的内容 / 错误信息
```

关键设计：

- **懒加载**：构造函数同步，技能 I/O 在首次 `runLoop` 时加载
- **缓存**：`discoverAll()` 结果缓存在内存中，`clearCache()` 可刷新
- **无循环引用**：Fork 模式下子 Agent 不继承 skill 工具自身
- **错误处理**：技能不存在时返回可用技能列表，辅助模型选择

### 9.12 文件结构变更

```
nodejs/
├── src/
│   ├── skills/                     # [新增] 技能系统模块
│   │   ├── index.ts                # 目录导出
│   │   ├── types.ts                # SkillMetadata, SkillInfo 等类型
│   │   ├── yaml.ts                 # 简易 YAML frontmatter 解析器
│   │   ├── loader.ts               # SkillLoader: 从磁盘发现 & 解析 SKILL.md
│   │   ├── substitution.ts         # 变量替换: $ARGUMENTS, ${CLAUDE_SKILL_DIR}, ${ENV:...}
│   │   └── tool.ts                 # SkillTool: ITool 实现，模型调用入口
│   ├── config.ts                   # [修改] AgentConfig 新增 enableSkills, skillsDir 等字段
│   ├── prompt.ts                   # [修改] buildSystemPrompt 接受 skills[], 注入 skill_listing
│   ├── agent.ts                    # [修改] initTools/runLoop 集成技能发现 + SkillTool 注册
│   └── index.ts                    # [修改] 导出 SkillLoader, SkillTool, SkillInfo 等
```
