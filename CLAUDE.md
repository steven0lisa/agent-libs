# agent-libs 项目指南

## 项目概述

agent-libs 是一个轻量级的多语言 Agent 库，支持 Rust / Go / Java / Node.js / Python 五种语言。核心能力是对接 Anthropic Messages API（以及兼容 API），实现标准的 agent-loop：模型可自主发起工具调用（tool_use），库负责执行工具并把结果（tool_result）回填给模型，直到产出最终回答。

## 语言实现成熟度

- **Python**：最完整的实现，功能最全，作为参考实现
- **Node.js**：完整实现，TypeScript/ESM
- **Go**：完整实现
- **Rust**：完整实现，async/tokio
- **Java**：基础实现

## 目录结构

```
agent-libs/
├── python/              # Python 实现（包名: agentlib）
│   ├── src/agentlib/    # 源码
│   │   ├── agent.py     # Agent 核心，生命周期管理（run/pause/resume/stop）
│   │   ├── client.py    # Anthropic API 客户端
│   │   ├── config.py    # 配置（AgentConfig, Pattern, OutputFormat）
│   │   ├── types.py     # 核心类型（Message, Event, EventType, ContentBlock 等）
│   │   ├── tool.py      # 工具基类（Tool, ToolContext, ToolResult）
│   │   ├── prompt.py    # 系统提示词构建
│   │   ├── compact.py   # Auto Compact 自动压缩对话历史
│   │   ├── tools/       # 内置工具实现
│   │   │   ├── read_file.py
│   │   │   ├── write_file.py
│   │   │   ├── update_file.py
│   │   │   ├── bash.py
│   │   │   ├── curl.py
│   │   │   └── subagent.py
│   │   ├── skills/      # Skills 系统
│   │   │   ├── types.py
│   │   │   ├── loader.py
│   │   │   ├── tool.py
│   │   │   ├── yaml.py
│   │   │   └── substitution.py
│   │   └── utils/
│   │       └── security.py  # 路径校验、安全策略
│   ├── tests/           # 测试
│   └── pyproject.toml   # Python >= 3.10, httpx, aiofiles
├── nodejs/              # Node.js/TypeScript 实现（包名: @zhangzichao2008/agent-lib）
│   ├── src/             # 源码（结构与 Python 对齐）
│   ├── tests/           # 测试
│   └── package.json     # Node >= 18, TypeScript
├── go/                  # Go 实现（模块: github.com/steven0lisa/agent-libs/go）
│   ├── agent.go         # Agent 核心
│   ├── client.go        # API 客户端
│   ├── config.go        # 配置
│   ├── types.go         # 类型定义
│   ├── tools/           # 内置工具
│   ├── skills/          # Skills 系统
│   ├── cmd/             # 命令行入口
│   └── pkg/             # 公共包
├── rust/                # Rust 实现（crate: agentlib）
│   ├── src/             # 源码（结构与 Python 对齐）
│   ├── tests/           # 集成测试
│   └── Cargo.toml       # Rust 2021, tokio, reqwest, serde
├── java/                # Java 实现（Maven: com.agentlib:agentlib）
│   ├── src/             # 源码
│   └── pom.xml          # Maven 配置
├── docs/PRD/            # 产品需求文档
│   ├── PRD.md           # 通用设计文档
│   ├── PRD-python.md    # Python 详细设计
│   ├── PRD-nodejs.md    # Node.js 详细设计
│   ├── PRD-go.md        # Go 详细设计
│   ├── PRD-java.md      # Java 详细设计
│   └── PRD-rust.md      # Rust 详细设计
├── examples/            # 各语言示例代码
│   ├── python/          # 基础用法、回调、生命周期、自定义工具、安全策略、子 Agent
│   ├── nodejs/
│   ├── go/
│   ├── java/
│   └── rust/
├── test/                # 跨语言集成测试
└── logs/                # 运行日志
```

## 核心架构

### Agent Loop（智能体循环）

这是库的核心机制，流程如下：

1. 用户输入 -> 构建请求 -> 调用 API
2. 接收模型响应，解析 content block（text / tool_use / thinking）
3. 若存在 tool_use -> 执行工具 -> 构造 tool_result -> 追加到历史 -> 回到步骤 2
4. 若无 tool_use（stop_reason=end_turn）-> 返回最终答案
5. 终止条件：模型完成 / max_turns / 用户取消 / max_duration_ms / 不可恢复错误

### 核心类型

- **Message**：`{ role: user | assistant, content: ContentBlock[] }`
- **ContentBlock**：TextBlock | ToolUseBlock | ToolResultBlock | ThinkingBlock
- **Event**：TURN_START / MESSAGE_START / MESSAGE_DELTA / THINKING_DELTA / MESSAGE_END / TOOL_USE_START / TOOL_USE_END / ERROR / COMPLETE / COMPACT
- **AgentConfig**：API 密钥、模型、工具列表、安全策略、压缩配置等

### Agent 生命周期

- IDLE -> RUNNING -> COMPLETED / TERMINATED
- 支持 pause/resume/stop 操作

## 关键特性

### 1. 内置工具

所有语言均实现以下工具：

- `read_file`：读取文件（受目录访问控制）
- `write_file`：写入/创建文件（受目录访问控制）
- `update_file`：替换文件内容（受目录访问控制）
- `bash`：执行 shell 命令（受白名单/黑名单策略控制）
- `curl`：HTTP 请求（受白名单/黑名单策略控制）
- `subagent`：子 Agent 调用

### 2. Auto Compact（自动压缩）

当对话历史增长接近模型上下文窗口限制时，自动压缩历史消息：

- 默认启用（`auto_compact=True`）
- 触发阈值：上下文窗口的 80%（200K * 0.8 = 160K tokens）
- 压缩方式：通过 API 生成摘要替换旧消息，保留最近的消息
- 安全机制：连续失败 3 次后停止尝试（熔断器）
- 至少保留最近 5 条消息
- 不拆分 tool_use / tool_result 消息对
- Token 估算：约 4 字符 = 1 token

### 3. 目录访问控制

配置 Agent 允许读写的目录列表：

- `work_dir` 始终可读写
- `allowed_read_dirs`：允许读取的额外目录
- `allowed_write_dirs`：允许写入的额外目录
- 空列表 = 仅 `work_dir` 可访问（默认行为，向后兼容）
- Skill 目录默认可读

### 4. Skills 系统

支持动态加载 Skill（YAML 格式定义），包含：

- SkillLoader：从目录加载 skill 定义
- SkillTool：将 skill 包装为工具
- 变量替换（substitution）：支持模板变量
- 支持 include_project_skills 从项目目录加载

### 5. 安全策略

- 路径校验：`resolve_safe_path` 确保文件操作不越界
- 命令过滤：`check_security_policy` 基于 whitelist/blacklist 模式匹配
- Pattern 支持 wildcard（fnmatch）和 regex 两种模式

## 环境变量

| 变量 | 必填 | 说明 |
|------|------|------|
| `ANTHROPIC_AUTH_TOKEN` | 二选一 | Anthropic API 密钥 |
| `ANTHROPIC_API_KEY` | 二选一 | Anthropic API 密钥 |
| `ANTHROPIC_BASE_URL` | 否 | API 基础 URL，默认 `https://api.anthropic.com` |
| `ANTHROPIC_MODEL` | 否 | 模型名称，默认 `claude-sonnet-4-6` |
| `AGENT_WORK_DIR` | 否 | 工作目录，文件/命令工具的沙箱基准 |

注意：Python 实现不主动读取环境变量，所有配置需调用方显式传入（支持多租户场景）。

## 开发与测试命令

### Python

```bash
cd python
pip install -e ".[dev]"     # 安装（含开发依赖）
pytest -q                    # 运行测试
pytest tests/test_agent.py   # 运行单个测试文件
pytest -v                    # 详细输出
```

依赖：Python >= 3.10, httpx >= 0.27, aiofiles >= 23, pytest, pytest-asyncio, pytest-httpx, respx

### Node.js

```bash
cd nodejs
npm ci                       # 安装依赖
npm run build                # TypeScript 编译
npm test                     # 运行测试
```

依赖：Node >= 18, TypeScript >= 5.4

### Go

```bash
cd go
go test ./...                # 运行所有测试
go test -v -run TestAgent    # 运行指定测试
```

### Rust

```bash
cd rust
cargo test                   # 运行测试
cargo test -- --nocapture    # 显示输出
```

依赖：Rust 2021 edition, tokio, reqwest, serde, async-trait, futures, thiserror, tracing

### Java

```bash
cd java
mvn test                     # 运行测试
```

## 编码规范

### 通用原则

- 各语言实现的模块结构和 API 设计保持一致（参考 Python 实现作为标准）
- 新增功能先在 Python 实现，验证后再移植到其他语言
- PRD 文档在 `docs/PRD/` 目录，新增功能必须先有 PRD

### Python 规范

- 使用 dataclass 定义数据类型，使用 frozen=True 保证不可变性
- 使用 async/await 进行异步操作
- 使用 logging 模块记录日志（不要用 print）
- 类型注解必须完整
- 测试使用 pytest + pytest-asyncio（asyncio_mode = "auto"）
- Mock API 使用 pytest-httpx / respx

### 新增工具的标准流程

1. 在 `tools/` 目录创建工具文件
2. 继承 `Tool` 基类，实现 `execute` 方法
3. 在 `__init__.py` 导出
4. 编写对应的测试文件
5. 更新各语言的 PRD 文档

### 新增语言的检查清单

确保以下模块都已实现：

- agent（核心循环 + 生命周期）
- client（API 客户端，支持流式）
- config（配置）
- types（消息、事件、内容块类型）
- tool（工具基类）
- prompt（系统提示词构建）
- compact（自动压缩）
- tools/（内置工具：read_file, write_file, update_file, bash, curl, subagent）
- skills/（Skills 系统）
- utils/security（路径校验、安全策略）

## 注意事项

- 不要提交 `.env`、密钥文件等敏感信息
- 日志目录 `logs/` 已在 `.gitignore` 中
- 测试时 Mock API 调用，不要使用真实 API Key
- Python 的 `asyncio_mode = "auto"` 意味着所有 async 测试函数自动被识别
