# Agent Lib PRD - 通用设计文档

## 1. 设计目标

实现一个轻量级的 Agent 库，支持多种编程语言（Rust/Go/Java/NodeJS/Python），核心能力是与 Anthropic API（或兼容 API，如智谱 GLM）交互，实现 agent-loop 机制——即模型可以自主调用工具、获取结果、继续推理，直到完成用户请求。

## 2. 核心概念

### 2.1 Agent Loop（智能体循环）

Agent Loop 是库的核心机制，参考 Claude Code 的实现：

```
┌─────────────────────────────────────────────────────────────┐
│                        Agent Loop                           │
│                                                             │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│  │ 用户输入  │───▶│ 构建请求  │───▶│ 调用API   │             │
│  └──────────┘    └──────────┘    └────┬─────┘             │
│                                       │                     │
│                                       ▼                     │
│                              ┌──────────────┐              │
│                              │ 接收模型响应  │              │
│                              └──────┬───────┘              │
│                                     │                       │
│                    包含tool_use?    │                       │
│                          ┌───是────┘                       │
│                          │                                  │
│                          ▼                                  │
│                   ┌────────────┐                            │
│                   │ 执行工具    │                            │
│                   └─────┬──────┘                            │
│                         │                                   │
│                         ▼                                   │
│              ┌────────────────────┐                        │
│              │ 构造tool_result消息 │                        │
│              │ 追加到消息历史      │                        │
│              └──────────┬─────────┘                        │
│                         │                                   │
│                         └────────────────┐                  │
│                                          │                  │
│                    否 ◀──────────────────┘                  │
│                     │                                       │
│                     ▼                                       │
│              ┌────────────┐                                 │
│              │ 返回最终答案│                                 │
│              └────────────┘                                 │
└─────────────────────────────────────────────────────────────┘
```

**循环流程详细说明**：

1. **初始化**：收集系统提示词(system prompt)、可用工具列表、用户初始消息
2. **发送请求**：将消息历史 + 工具定义 + 系统提示词组装成 API 请求，发送到 Anthropic Messages API
3. **接收响应**：流式或非流式接收 assistant 的响应内容
4. **解析内容块**：响应由多个 content block 组成，类型包括：
   - `text`：普通文本输出
   - `tool_use`：工具调用请求（含工具名、工具ID、参数）
   - `thinking`：模型推理过程（如 Claude 3.7 Sonnet）
5. **判断循环**：
   - 若存在 `tool_use` block → 执行对应工具 → 将结果包装为 `tool_result` block → 作为 user 消息追加到历史 → 回到步骤2
   - 若无 `tool_use` 且 stop_reason = `end_turn` → 循环结束，返回最终答案
6. **终止条件**：
   - 模型不再请求工具调用（stop_reason = end_turn）
   - 达到最大轮数限制（max_turns）
   - 用户主动取消（abort signal）
   - 达到运行时长限制（max_duration_ms）
   - 发生不可恢复的错误

### 2.2 消息（Message）

消息是 Agent 与模型交互的基本单元，遵循 Anthropic Messages API 格式。

**消息结构**：
```
Message {
  role: "user" | "assistant"
  content: ContentBlock[]
}
```

**ContentBlock 类型**：

| 类型 | 方向 | 说明 |
|------|------|------|
| `text` | user/assistant | 纯文本内容 |
| `tool_use` | assistant | 模型请求调用工具，包含 name, id, input |
| `tool_result` | user | 工具执行结果，包含 tool_use_id, content, is_error |

**消息流转示例**：
```
// Turn 1: 用户输入
{ role: "user", content: [{ type: "text", text: "帮我读取文件 a.txt" }] }

// Turn 1: 模型响应（请求工具调用）
{ role: "assistant", content: [
  { type: "tool_use", name: "read_file", id: "tu_01", input: { file_path: "/tmp/a.txt" } }
]}

// Turn 2: 用户消息（工具结果）
{ role: "user", content: [
  { type: "tool_result", tool_use_id: "tu_01", content: "文件内容: Hello World" }
]}

// Turn 2: 模型响应（最终答案）
{ role: "assistant", content: [
  { type: "text", text: "文件内容是 'Hello World'" }
]}
```

### 2.3 工具（Tool）

工具是 Agent 扩展能力的核心机制。每个工具由大模型根据描述和参数 schema 自主决定何时调用。

**工具定义**：
```
Tool {
  name: string                    // 工具唯一标识符，用于模型识别
  description: string             // 工具功能描述，模型据此判断何时调用
  input_schema: JSONSchema        // 参数 JSON Schema，模型据此构造参数
  handler: function               // 实际执行逻辑
}
```

**工具与模型交互流程**：
1. 启动时：Agent 将所有工具的 `name`, `description`, `input_schema` 注册到 API 请求中
2. 推理时：模型根据当前任务和工具描述，决定调用哪个工具、传入什么参数
3. 调用时：Agent 解析模型输出的 `tool_use` block，匹配到对应工具，执行 handler
4. 返回时：Agent 将 handler 的返回值包装为 `tool_result` block，送回模型

**预定义工具**：

| 工具名 | 功能 | 输入参数 | 并发安全 |
|--------|------|----------|----------|
| `read_file` | 读取文件内容 | file_path, offset?, limit? | 是 |
| `write_file` | 写入/创建文件 | file_path, content | 否 |
| `update_file` | 替换文件内容 | file_path, old_string, new_string, replace_all? | 否 |
| `bash` | 执行 shell 命令 | command, timeout?, description? | 视命令而定 |
| `curl` | 发起 HTTP 请求 | url, method?, headers?, body?, timeout? | 是 |

### 2.4 自定义扩展（Custom Extension）

除预定义工具外，Agent 支持运行时注册自定义工具，允许 Agent 根据业务场景动态扩展能力。

**扩展方式**：
1. **编程注册**：通过代码直接注册自定义工具到 Agent 实例
2. **配置文件加载**：从配置文件（如 JSON/YAML）中加载工具定义和映射

**自定义工具与预定义工具的区别**：
- 预定义工具由库提供默认实现，开箱即用
- 自定义工具由使用者提供实现，通过统一的注册接口接入 Agent Loop

**自定义工具的完整生命周期**：
```
注册（register_tool）
  → 纳入工具列表（随每次请求发送给模型）
  → 模型决定调用（输出 tool_use）
  → Agent 路由到自定义 handler
  → 执行并返回结果
  → 包装为 tool_result 送回模型
```

### 2.5 配置（Config）

Agent 运行所需的外部配置：

| 配置项 | 环境变量 | 说明 |
|--------|----------|------|
| base_url | ANTHROPIC_BASE_URL | API 基础地址，支持自定义兼容服务 |
| api_key | ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY | API 认证密钥 |
| model | ANTHROPIC_MODEL | 模型名称，如 claude-sonnet-4, GLM-5.1 |
| work_dir | AGENT_WORK_DIR | 工作目录，所有文件操作和 bash 命令的基准路径 |
| max_tokens | AGENT_MAX_TOKENS | 单次请求最大输出 token 数 |
| max_turns | AGENT_MAX_TURNS | 最大对话轮数，防止无限循环 |
| max_duration_ms | AGENT_MAX_DURATION_MS | Agent 总运行时长限制(ms)，默认无限制(0) |
| system_prompt | - | 系统提示词，追加到默认提示词 |
| output_format | - | 输出格式: "text"(默认) 或 "json" |
| callback | - | 响应回调函数，每步事件触发 |
| auto_compact | - | 是否启用自动压缩，默认 true |
| context_window_size | - | 模型上下文窗口大小，默认 200000 |
| auto_compact_threshold_pct | - | 自动压缩触发阈值百分比，默认 0.8（80%） |
| allowed_read_dirs | - | 允许读取的目录列表（空列表表示仅 work_dir） |
| allowed_write_dirs | - | 允许写入的目录列表（空列表表示仅 work_dir） |

**配置优先级**：构造函数参数 > 环境变量 > 默认值

### 2.6 安全策略（SecurityPolicy）

安全策略用于控制工具的执行权限，支持白名单和黑名单机制。

**Pattern 匹配规则**：
- `wildcard`：通配符模式，如 `*.example.com`, `git *`, `ls *`
- `regex`：正则表达式模式，如 `^git\s+(status|log|diff)`, `^https://.*\.example\.com.*`

**匹配优先级**：白名单 > 黑名单。先检查白名单（若匹配则放行），再检查黑名单（若匹配则拒绝），无匹配则默认放行（或拒绝，可配置）。

**应用到工具**：
- `BashTool`：对 command 字符串进行模式匹配
- `CurlTool`：对 URL 进行模式匹配

## 3. API 设计（语言无关概念）

### 3.1 核心类/模块

#### Agent

Agent 是库的入口类，封装整个 agent-loop 的生命周期。

```
class Agent {
  // 构造
  constructor(config: AgentConfig)

  // 核心方法
  run(input: string): AsyncIterator<Event>
  run_sync(input: string): Result
  chat(messages: Message[]): AsyncIterator<Event>
  stop(): void

  // 工具管理
  register_tool(tool: Tool): void
  unregister_tool(name: string): void
  list_tools(): Tool[]

  // 状态
  get_message_history(): Message[]
  clear_history(): void
}
```

#### AgentConfig

```
struct AgentConfig {
  base_url: string        // 默认: "https://api.anthropic.com"
  api_key: string
  model: string           // 默认: "claude-sonnet-4-6"
  work_dir: string        // 默认: process.cwd() / os.getcwd() / std::env::current_dir()
  max_tokens: int         // 默认: 8192
  max_turns: int          // 默认: 100
  max_duration_ms: int    // 默认: 0 (无限制)
  system_prompt: string?  // 追加到默认系统提示词
  timeout_ms: int         // 默认: 120000
  stream: bool            // 默认: true
  tools: Tool[]           // 初始自定义工具列表
  output_format: string   // 默认: "text", 可选: "json"
  callback: function?     // 响应回调: (Event) -> void
  // 安全策略
  bash_whitelist: Pattern[]  // Bash 白名单
  bash_blacklist: Pattern[]  // Bash 黑名单
  curl_whitelist: Pattern[]  // Curl 白名单
  curl_blacklist: Pattern[]  // Curl 黑名单
  // Auto Compact
  auto_compact: bool         // 默认: true，是否启用自动上下文压缩
  context_window_size: int   // 默认: 200000，模型上下文窗口大小
  auto_compact_threshold_pct: float  // 默认: 0.8，触发阈值百分比
  // 目录访问控制
  allowed_read_dirs: string[]   // 允许读取的目录（空=仅 work_dir）
  allowed_write_dirs: string[]  // 允许写入的目录（空=仅 work_dir）
}
```

#### Pattern

```
struct Pattern {
  pattern: string
  type: "wildcard" | "regex"   // 匹配模式类型
}

// 匹配方法
match(text: string, pattern: Pattern): bool
```

#### Tool

```
interface Tool {
  name: string
  description: string
  input_schema: JSONSchema
  handler: (input: object, context: ToolContext) -> ToolResult
  // 可选
  is_read_only: bool      // 默认: false，标记是否只读（影响并发策略）
}

struct ToolContext {
  work_dir: string
  message_history: Message[]
}

type ToolResult = string | { content: string, is_error: bool }
```

### 3.2 事件流（Event Stream）

`run()` 方法返回异步事件流，让调用者可以实时观察 Agent 的执行过程。

```
enum EventType {
  MESSAGE_START        // 开始接收模型响应
  MESSAGE_DELTA        // 收到内容片段（流式）
  MESSAGE_END          // 模型响应结束
  TOOL_USE_START       // 模型请求调用工具
  TOOL_USE_END         // 工具执行完成
  TOOL_RESULT          // 工具结果已生成
  THINKING_DELTA       // 收到思考内容（流式）
  ERROR                // 发生错误
  COMPLETE             // 整个任务完成
  TURN_START           // 新一轮对话开始
  COMPACT              // 上下文压缩事件
}

interface Event {
  type: EventType
  data: object           // 根据 type 不同而变化
}
```

### 3.3 回调机制（Callback）

Agent 支持注册回调函数，在每次事件发生时触发，便于日志打印和进度监控。

```
// 回调函数签名
type Callback = (event: Event) -> void

// 使用方式
agent = new Agent({
  ...config,
  callback: (event) => {
    console.log(`[${event.type}]`, event.data)
  }
})
```

回调在事件流产生时同步触发，不影响事件流本身。

### 3.4 输出格式（OutputFormat）

Agent 支持自定义最终输出结构。

```
enum OutputFormat {
  TEXT   // 默认，返回纯文本
  JSON   // 返回结构化 JSON
}

// TEXT 模式下 Event::COMPLETE 的数据
{ final_content: string }

// JSON 模式下 Event::COMPLETE 的数据
{ final_content: string, tool_calls: [...], message_history: [...] }
```

### 3.5 系统提示词模板

默认系统提示词（可被 user 的 system_prompt 追加）：

```
You are a helpful software engineering assistant. You have access to tools that let you interact with the file system and execute commands.

## Tool Use

You will be provided with a set of tools. When you need to perform an action, use the appropriate tool by outputting a tool_use block.

Tool use format:
- Each tool has a name and input parameters defined by a JSON schema
- To use a tool, output a content block with type "tool_use"
- The tool_use block must include: name (tool name), id (unique identifier), input (parameters as JSON object)

After using a tool, the user message will contain a tool_result block with the execution result.
- If the result indicates success, continue with your analysis or next steps
- If the result indicates an error, analyze the error and decide whether to retry, use a different approach, or ask the user for clarification

## Available Tools

{tool_descriptions}  // 动态生成，根据当前注册的工具列表

## File Operations

When working with files:
- Always read a file before editing it
- When editing files, use update_file tool with old_string/new_string for precise changes
- Use write_file to create new files
- Use read_file to inspect file contents

## Command Execution

When executing commands via bash:
- Prefer read-only commands for exploration (ls, grep, find, cat, etc.)
- Be careful with destructive commands (rm, dd, etc.)
- Always describe what the command does before executing it
```

### 3.6 文件安全（File Sandbox）

所有文件操作（read_file, write_file, update_file）被限制在允许的目录内：

1. **路径解析**：所有相对路径基于 `work_dir` 解析
2. **允许目录检查**：
   - `allowed_read_dirs`：允许读取的目录列表（空列表表示仅 `work_dir`）
   - `allowed_write_dirs`：允许写入的目录列表（空列表表示仅 `work_dir`）
   - Skill 目录默认可读（不受 `allowed_read_dirs` 限制）
   - `work_dir` 始终可读写
3. **越界检查**：使用 `path.resolve()` 后检查是否为任一允许目录的子路径
4. **禁止符号链接逃逸**：解析后检查真实路径（canonical path）
5. **失败处理**：越界时返回 tool_result（is_error=true），不抛出异常

```
resolve_and_check(file_path, work_dir, allowed_dirs):
  resolved = work_dir.resolve(file_path).canonical()
  // 检查是否在 work_dir 或任一 allowed_dirs 内
  if is_subpath(resolved, work_dir):
    return resolved
  for dir in allowed_dirs:
    if is_subpath(resolved, dir):
      return resolved
  return ERROR("Path escapes allowed directories")
```

### 3.6.1 自动上下文压缩（Auto Compact）

当对话历史增长到接近模型上下文窗口限制时，自动压缩历史消息以保持系统性能。参考 Claude Code 的 auto compact 机制。

**触发条件**：

```
estimated_tokens(message_history) >= context_window_size * auto_compact_threshold_pct
```

**压缩流程**：

```
┌───────────────────────────────────────────────────────┐
│                  Auto Compact Flow                     │
│                                                       │
│  ┌─────────────────┐                                  │
│  │ 估算消息历史 tokens│                                 │
│  └────────┬────────┘                                  │
│           │                                            │
│           ▼                                            │
│  ┌─────────────────────┐                              │
│  │ 超过阈值？           │                              │
│  └────┬──────────┬─────┘                              │
│      否          是                                    │
│       │           │                                    │
│       ▼           ▼                                    │
│  继续执行    ┌───────────────┐                         │
│             │ 保留最近N条消息 │                         │
│             └───────┬───────┘                         │
│                     │                                  │
│                     ▼                                  │
│            ┌────────────────┐                          │
│            │ API 生成摘要    │                          │
│            │ 压缩旧消息      │                          │
│            └───────┬────────┘                         │
│                    │                                   │
│                    ▼                                   │
│            ┌──────────────────┐                       │
│            │ 替换消息历史      │                       │
│            │ [摘要] + [最近消息]│                      │
│            └──────────────────┘                       │
└───────────────────────────────────────────────────────┘
```

**配置选项**：

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `auto_compact` | `true` | 是否启用自动压缩 |
| `context_window_size` | `200000` | 模型上下文窗口大小（tokens） |
| `auto_compact_threshold_pct` | `0.8` | 触发阈值百分比（0.0-1.0） |

**压缩策略**：

1. **Token 估算**：使用启发式方法估算消息历史的 token 数（~4字符/token）
2. **保留策略**：保留最近的消息（至少保留最近 5 条），确保工具调用/结果配对完整性
3. **摘要生成**：使用同一 API 调用生成压缩摘要，替换旧消息
4. **断路器**：最多连续 3 次压缩失败后停止尝试，避免无限重试
5. **事件通知**：压缩时发出 `COMPACT` 事件，调用者可通过事件流感知

### 3.7 Bash 安全策略

BashTool 支持白名单/黑名单控制命令执行：

```
BashTool {
  whitelist: Pattern[]   // 匹配则自动放行
  blacklist: Pattern[]   // 匹配则拒绝执行
  default_action: "allow" | "deny"   // 无匹配时的默认行为
}

执行前检查：
1. 遍历 whitelist，若 command 匹配任一 pattern → 放行
2. 遍历 blacklist，若 command 匹配任一 pattern → 拒绝
3. 无匹配时按 default_action 处理（默认 allow）
```

### 3.8 Curl 安全策略

CurlTool 支持白名单/黑名单控制 HTTP 请求：

```
CurlTool {
  whitelist: Pattern[]   // URL 匹配则自动放行
  blacklist: Pattern[]   // URL 匹配则拒绝执行
  follow_redirects: bool     // 默认: true
  verify_ssl: bool           // 默认: true
  default_action: "allow" | "deny"
}

执行前检查：
1. 解析 URL，提取 host 和完整 URL
2. 遍历 whitelist，若 URL 匹配 → 放行
3. 遍历 blacklist，若 URL 匹配 → 拒绝
4. 无匹配时按 default_action 处理
```

### 3.9 错误处理策略

| 错误类型 | 处理方式 |
|----------|----------|
| API 请求失败（网络/认证） | 重试3次（指数退避），最终抛出 |
| 工具执行失败 | 将错误信息包装为 tool_result（is_error=true）送回模型 |
| 工具不存在 | tool_result 中返回错误，模型决定下一步 |
| 参数校验失败 | 同工具执行失败 |
| 路径越界（文件操作） | tool_result 中返回错误 |
| 安全策略拒绝 | tool_result 中返回拒绝原因 |
| 超时 | 中断当前轮次，返回超时错误 |
| 达到 max_turns | 强制终止，返回当前状态 |
| 达到 max_duration_ms | 强制终止，返回当前状态 |

### 3.10 并发策略

工具执行支持两种模式：

**串行执行（Sequential）**：适用于写操作或有依赖关系的工具调用
- write_file, update_file 等修改类工具
- 工具间有数据依赖的情况

**并发执行（Concurrent）**：适用于独立的读操作
- read_file, bash（只读命令）, curl 等
- 多个 tool_use 块在同一条 assistant 消息中时，若都是只读工具则并发执行

判断逻辑：根据工具的 `is_read_only` 属性分组，只读工具组内并发，写工具串行。

## 4. 各语言实现原则

每种语言的实现需要遵循以下原则：

1. **API 语义一致**：核心概念（Agent, Tool, Message, Event）在所有语言中语义等价
2. **惯用设计**：遵循各语言的惯用编程风格（如 Rust 的 trait、Go 的 interface、Python 的 dataclass）
3. **事件驱动**：`run()` 方法返回异步事件流/迭代器/通道，支持流式观察
4. **可测试性**：核心逻辑不依赖外部 IO，通过接口抽象 HTTP 客户端和文件系统
5. **无状态优先**：Agent 实例持有状态（消息历史），但不依赖全局状态

## 5. 目录结构

```
agent-libs/
├── docs/
│   ├── PRD.md              # 本文件：通用设计文档
│   ├── PRD-rust.md         # Rust 语言特定设计
│   ├── PRD-go.md           # Go 语言特定设计
│   ├── PRD-java.md         # Java 语言特定设计
│   ├── PRD-nodejs.md       # NodeJS 语言特定设计
│   └── PRD-python.md       # Python 语言特定设计
├── rust/                   # Rust 实现
├── go/                     # Go 实现
├── java/                   # Java 实现
├── nodejs/                 # NodeJS 实现
├── python/                 # Python 实现
└── README.md
```
