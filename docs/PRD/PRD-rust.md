# PRD - Rust 实现

## 1. 语言特性映射

| 通用概念 | Rust 实现 |
|----------|-----------|
| Agent | `struct Agent` |
| Tool | `trait Tool` + `struct ToolDefinition` |
| Message | `enum Message` (role + content) |
| ContentBlock | `enum ContentBlock` |
| Event | `enum Event` |
| Config | `struct AgentConfig` |
| 异步 | `async`/`.await` + `tokio` |
| 事件流 | `tokio::sync::mpsc::Receiver<Event>` 或 `impl Stream<Item=Event>` |
| 错误处理 | `Result<T, AgentError>` |
| JSON | `serde_json::Value` |
| HTTP | `reqwest` |

## 2. 核心类型设计

### 2.1 ContentBlock

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        name: String,
        id: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}
```

### 2.2 Message

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}
```

### 2.3 Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;

    /// 是否只读操作，影响并发策略
    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError>;
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub work_dir: std::path::PathBuf,
    pub message_history: Vec<Message>,
}

pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

pub struct ToolError(pub String);
```

### 2.4 Event

```rust
#[derive(Debug, Clone)]
pub enum Event {
    TurnStart { turn: usize },
    MessageStart,
    MessageDelta { text: String },
    ThinkingDelta { thinking: String },
    MessageEnd,
    ToolUseStart { name: String, id: String, input: serde_json::Value },
    ToolUseEnd { name: String, id: String, result: ToolResult },
    ToolResult { tool_use_id: String, content: String },
    Error { message: String },
    Complete { final_content: String },
}
```

### 2.5 AgentConfig

```rust
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub base_url: String,      // 默认: "https://api.anthropic.com"
    pub api_key: String,
    pub model: String,         // 默认: "claude-sonnet-4-6"
    pub work_dir: std::path::PathBuf,
    pub max_tokens: u32,       // 默认: 8192
    pub max_turns: usize,      // 默认: 100
    pub system_prompt: Option<String>,
    pub timeout_ms: u64,       // 默认: 120000
    pub stream: bool,          // 默认: true
}

impl Default for AgentConfig {
    fn default() -> Self { ... }
}
```

### 2.6 Agent

```rust
pub struct Agent {
    config: AgentConfig,
    client: reqwest::Client,
    tools: Vec<Box<dyn Tool>>,
    message_history: Vec<Message>,
    turn_count: usize,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self;

    /// 注册自定义工具
    pub fn register_tool(&mut self, tool: Box<dyn Tool>);
    pub fn unregister_tool(&mut self, name: &str);
    pub fn list_tools(&self) -> Vec<&dyn Tool>;

    /// 核心方法：运行 agent loop，返回事件流
    pub async fn run(&mut self, input: &str) -> Result<mpsc::Receiver<Event>, AgentError>;

    /// 同步版本（阻塞当前线程）
    pub fn run_blocking(&mut self, input: &str) -> Result<Vec<Event>, AgentError>;

    /// 以已有消息历史继续对话
    pub async fn chat(&mut self, messages: Vec<Message>) -> Result<mpsc::Receiver<Event>, AgentError>;

    pub fn get_message_history(&self) -> &[Message];
    pub fn clear_history(&mut self);
}
```

## 3. 预定义工具实现

### 3.1 ReadFileTool

```rust
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }
    fn description(&self) -> &str { "Read file contents. Supports text, images, PDFs." }
    fn is_read_only(&self) -> bool { true }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string", "description": "Absolute or relative path to the file" },
                "offset": { "type": "integer", "description": "Line number to start reading from" },
                "limit": { "type": "integer", "description": "Maximum number of lines to read" }
            },
            "required": ["file_path"]
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"].as_str().ok_or(ToolError("file_path required".into()))?;
        let path = resolve_path(file_path, &ctx.work_dir);
        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| ToolError(format!("Failed to read file: {}", e)))?;
        Ok(ToolResult { content, is_error: false })
    }
}
```

### 3.2 WriteFileTool

```rust
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }
    fn description(&self) -> &str { "Write content to a file. Creates the file if it doesn't exist. Overwrites if it does." }
    fn is_read_only(&self) -> bool { false }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["file_path", "content"]
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"].as_str().ok_or(ToolError("file_path required".into()))?;
        let content = input["content"].as_str().unwrap_or("");
        let path = resolve_path(file_path, &ctx.work_dir);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| ToolError(format!("Failed to create directory: {}", e)))?;
        }

        tokio::fs::write(&path, content).await
            .map_err(|e| ToolError(format!("Failed to write file: {}", e)))?;

        Ok(ToolResult {
            content: format!("File written successfully: {}", path.display()),
            is_error: false,
        })
    }
}
```

### 3.3 UpdateFileTool

```rust
pub struct UpdateFileTool;

#[async_trait]
impl Tool for UpdateFileTool {
    fn name(&self) -> &str { "update_file" }
    fn description(&self) -> &str { "Update a file by replacing old_string with new_string." }
    fn is_read_only(&self) -> bool { false }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string" },
                "old_string": { "type": "string", "description": "The text to replace" },
                "new_string": { "type": "string", "description": "The replacement text" },
                "replace_all": { "type": "boolean", "default": false, "description": "Replace all occurrences" }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"].as_str().ok_or(ToolError("file_path required".into()))?;
        let old_str = input["old_string"].as_str().ok_or(ToolError("old_string required".into()))?;
        let new_str = input["new_string"].as_str().ok_or(ToolError("new_string required".into()))?;
        let replace_all = input["replace_all"].as_bool().unwrap_or(false);

        let path = resolve_path(file_path, &ctx.work_dir);
        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| ToolError(format!("Failed to read file: {}", e)))?;

        let new_content = if replace_all {
            content.replace(old_str, new_str)
        } else {
            content.replacen(old_str, new_str, 1)
        };

        if new_content == content {
            return Err(ToolError("old_string not found in file".into()));
        }

        tokio::fs::write(&path, new_content).await
            .map_err(|e| ToolError(format!("Failed to write file: {}", e)))?;

        Ok(ToolResult {
            content: format!("File updated successfully: {}", path.display()),
            is_error: false,
        })
    }
}
```

### 3.4 BashTool

```rust
pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }
    fn description(&self) -> &str { "Execute a shell command in the working directory." }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" },
                "description": { "type": "string", "description": "A brief description of what the command does" },
                "timeout": { "type": "integer", "description": "Timeout in milliseconds", "default": 120000 }
            },
            "required": ["command"]
        })
    }

    fn is_read_only(&self) -> bool {
        // 运行时根据命令判断，默认 false
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let command = input["command"].as_str().ok_or(ToolError("command required".into()))?;
        let timeout_ms = input["timeout"].as_u64().unwrap_or(120000);

        let output = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&ctx.work_dir)
                .output()
        ).await
            .map_err(|_| ToolError("Command timed out".into()))?
            .map_err(|e| ToolError(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let content = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\n[stderr]\n{}", stdout, stderr)
        };

        Ok(ToolResult {
            content,
            is_error: !output.status.success(),
        })
    }
}
```

### 3.5 CurlTool

```rust
pub struct CurlTool;

#[async_trait]
impl Tool for CurlTool {
    fn name(&self) -> &str { "curl" }
    fn description(&self) -> &str { "Make an HTTP request." }
    fn is_read_only(&self) -> bool { true }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" },
                "method": { "type": "string", "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"], "default": "GET" },
                "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                "body": { "type": "string" },
                "timeout": { "type": "integer", "default": 30000 }
            },
            "required": ["url"]
        })
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        // 使用 reqwest 发送 HTTP 请求
        // ...
    }
}
```

## 4. API 调用层

### 4.1 AnthropicClient

```rust
pub struct AnthropicClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    max_tokens: u32,
}

impl AnthropicClient {
    pub fn new(config: &AgentConfig) -> Self { ... }

    /// 发送流式请求，返回 SSE 事件流
    pub async fn stream_messages(
        &self,
        messages: Vec<Message>,
        system: Vec<String>,
        tools: Vec<ToolDefinition>,
    ) -> Result<impl Stream<Item = Result<StreamEvent, ApiError>>, ApiError>;

    /// 发送非流式请求
    pub async fn send_messages(
        &self,
        messages: Vec<Message>,
        system: Vec<String>,
        tools: Vec<ToolDefinition>,
    ) -> Result<ApiResponse, ApiError>;
}
```

### 4.2 流式解析

```rust
#[derive(Debug)]
pub enum StreamEvent {
    MessageStart { message: ApiMessage },
    ContentBlockStart { index: usize, content_block: ContentBlock },
    ContentBlockDelta { index: usize, delta: ContentDelta },
    ContentBlockStop { index: usize },
    MessageDelta { stop_reason: Option<String>, usage: Option<Usage> },
    MessageStop,
    Ping,
}

#[derive(Debug)]
pub enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
    SignatureDelta { signature: String },
}
```

## 5. Agent Loop 实现

```rust
impl Agent {
    pub async fn run(&mut self, input: &str) -> Result<mpsc::Receiver<Event>, AgentError> {
        let (tx, rx) = mpsc::channel(100);

        // 初始用户消息
        let user_msg = Message {
            role: Role::User,
            content: vec![ContentBlock::Text { text: input.to_string() }],
        };
        self.message_history.push(user_msg);

        // 启动 agent loop
        let mut agent = self.clone(); // 或传递必要的字段
        tokio::spawn(async move {
            if let Err(e) = agent.run_loop(tx.clone()).await {
                let _ = tx.send(Event::Error { message: e.to_string() }).await;
            }
            let _ = tx.send(Event::Complete { final_content: String::new() }).await;
        });

        Ok(rx)
    }

    async fn run_loop(&mut self, tx: mpsc::Sender<Event>) -> Result<(), AgentError> {
        let system_prompt = build_system_prompt(&self.tools, self.config.system_prompt.as_deref());

        while self.turn_count < self.config.max_turns {
            self.turn_count += 1;
            tx.send(Event::TurnStart { turn: self.turn_count }).await.ok();

            // 1. 调用 API
            let tool_defs = self.tools.iter().map(|t| ToolDefinition::from(&**t)).collect();

            let stream = self.client.stream_messages(
                self.message_history.clone(),
                vec![system_prompt.clone()],
                tool_defs,
            ).await?;

            // 2. 流式接收响应
            let mut assistant_content: Vec<ContentBlock> = Vec::new();
            let mut tool_use_blocks: Vec<ContentBlock> = Vec::new();

            tx.send(Event::MessageStart).await.ok();

            tokio::pin!(stream);
            while let Some(event) = stream.next().await {
                match event? {
                    StreamEvent::ContentBlockStart { content_block, .. } => {
                        assistant_content.push(content_block.clone());
                    }
                    StreamEvent::ContentBlockDelta { delta, .. } => {
                        match delta {
                            ContentDelta::TextDelta { text } => {
                                tx.send(Event::MessageDelta { text: text.clone() }).await.ok();
                            }
                            ContentDelta::ThinkingDelta { thinking } => {
                                tx.send(Event::ThinkingDelta { thinking }).await.ok();
                            }
                            ContentDelta::InputJsonDelta { partial_json } => {
                                // 累积到对应的 tool_use block
                            }
                            _ => {}
                        }
                    }
                    StreamEvent::ContentBlockStop { .. } => {}
                    StreamEvent::MessageStop => break,
                    _ => {}
                }
            }

            tx.send(Event::MessageEnd).await.ok();

            // 3. 提取 tool_use blocks
            for block in &assistant_content {
                if let ContentBlock::ToolUse { .. } = block {
                    tool_use_blocks.push(block.clone());
                }
            }

            // 4. 将 assistant 消息加入历史
            self.message_history.push(Message {
                role: Role::Assistant,
                content: assistant_content,
            });

            // 5. 若无 tool_use，任务完成
            if tool_use_blocks.is_empty() {
                let final_text = extract_text_from_message(self.message_history.last().unwrap());
                tx.send(Event::Complete { final_content: final_text }).await.ok();
                return Ok(());
            }

            // 6. 执行工具
            let tool_results = self.execute_tools(&tool_use_blocks, &tx).await?;

            // 7. 将 tool_result 加入历史
            self.message_history.push(Message {
                role: Role::User,
                content: tool_results,
            });
        }

        Err(AgentError::MaxTurnsReached)
    }

    async fn execute_tools(
        &self,
        tool_uses: &[ContentBlock],
        tx: &mpsc::Sender<Event>,
    ) -> Result<Vec<ContentBlock>, AgentError> {
        let mut results = Vec::new();

        // 分组：只读工具并发，写工具串行
        let mut read_only_group: Vec<&ContentBlock> = Vec::new();
        let mut write_group: Vec<&ContentBlock> = Vec::new();

        for block in tool_uses {
            if let ContentBlock::ToolUse { name, .. } = block {
                let tool = self.find_tool(name).ok_or(AgentError::ToolNotFound(name.clone()))?;
                if tool.is_read_only() {
                    read_only_group.push(block);
                } else {
                    write_group.push(block);
                }
            }
        }

        // 并发执行只读工具
        let read_futures = read_only_group.iter().map(|block| async {
            if let ContentBlock::ToolUse { name, id, input } = block {
                tx.send(Event::ToolUseStart { name: name.clone(), id: id.clone(), input: input.clone() }).await.ok();

                let tool = self.find_tool(name).unwrap();
                let ctx = ToolContext {
                    work_dir: self.config.work_dir.clone(),
                    message_history: self.message_history.clone(),
                };

                let result = tool.call(input.clone(), &ctx).await;
                match result {
                    Ok(r) => {
                        tx.send(Event::ToolUseEnd { name: name.clone(), id: id.clone(), result: r.clone() }).await.ok();
                        ContentBlock::ToolResult {
                            tool_use_id: id.clone(),
                            content: r.content,
                            is_error: Some(r.is_error),
                        }
                    }
                    Err(e) => ContentBlock::ToolResult {
                        tool_use_id: id.clone(),
                        content: e.0,
                        is_error: Some(true),
                    }
                }
            } else {
                unreachable!()
            }
        });

        results.extend(futures::future::join_all(read_futures).await);

        // 串行执行写工具
        for block in write_group {
            if let ContentBlock::ToolUse { name, id, input } = block {
                // ... 类似上面，但串行
            }
        }

        Ok(results)
    }
}
```

## 6. 自定义工具注册

```rust
// 定义自定义工具
struct MyCustomTool;

#[async_trait]
impl Tool for MyCustomTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Does something custom" }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "param1": { "type": "string" }
            },
            "required": ["param1"]
        })
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let param1 = input["param1"].as_str().unwrap_or("");
        Ok(ToolResult {
            content: format!("Processed: {}", param1),
            is_error: false,
        })
    }
}

// 使用
let mut agent = Agent::new(config);
agent.register_tool(Box::new(MyCustomTool));
```

## 7. 目录结构

```
rust/
├── Cargo.toml
├── src/
│   ├── lib.rs              # 模块导出
│   ├── agent.rs            # Agent 核心实现
│   ├── config.rs           # AgentConfig
│   ├── types.rs            # Message, ContentBlock, Role, Event
│   ├── tool.rs             # Tool trait, ToolContext, ToolResult, ToolError
│   ├── tools/              # 预定义工具
│   │   ├── mod.rs
│   │   ├── read_file.rs
│   │   ├── write_file.rs
│   │   ├── update_file.rs
│   │   ├── bash.rs
│   │   └── curl.rs
│   ├── client.rs           # Anthropic API 客户端
│   ├── stream.rs           # SSE 流式解析
│   ├── prompt.rs           # 系统提示词构建
│   └── error.rs            # 错误类型定义
└── tests/
    ├── integration_tests.rs
    └── fixtures/
```

## 8. 依赖

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
futures = "0.3"
thiserror = "1.0"
```
