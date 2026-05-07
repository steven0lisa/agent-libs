# PRD - Go 实现

## 1. 语言特性映射

| 通用概念 | Go 实现 |
|----------|---------|
| Agent | `struct Agent` |
| Tool | `interface Tool` |
| Message | `struct Message` + `type Role string` |
| ContentBlock | `interface ContentBlock` + 具体类型 |
| Event | `struct Event` + `type EventType string` |
| Config | `struct Config` |
| 异步 | goroutine + channel |
| 事件流 | `<-chan Event` (channel) |
| 错误处理 | `(T, error)` 返回值 |
| JSON | `map[string]any` / `json.RawMessage` |
| HTTP | `net/http` |

## 2. 核心类型设计

### 2.1 ContentBlock

```go
type ContentBlockType string

const (
    BlockText        ContentBlockType = "text"
    BlockToolUse     ContentBlockType = "tool_use"
    BlockToolResult  ContentBlockType = "tool_result"
    BlockThinking    ContentBlockType = "thinking"
)

type ContentBlock interface {
    BlockType() ContentBlockType
}

type TextBlock struct {
    Text string `json:"text"`
}

func (t TextBlock) BlockType() ContentBlockType { return BlockText }

type ToolUseBlock struct {
    Name  string          `json:"name"`
    ID    string          `json:"id"`
    Input json.RawMessage `json:"input"`
}

func (t ToolUseBlock) BlockType() ContentBlockType { return BlockToolUse }

type ToolResultBlock struct {
    ToolUseID string  `json:"tool_use_id"`
    Content   string  `json:"content"`
    IsError   *bool   `json:"is_error,omitempty"`
}

func (t ToolResultBlock) BlockType() ContentBlockType { return BlockToolResult }

type ThinkingBlock struct {
    Thinking  string `json:"thinking"`
    Signature string `json:"signature,omitempty"`
}

func (t ThinkingBlock) BlockType() ContentBlockType { return BlockThinking }
```

### 2.2 Message

```go
type Role string

const (
    RoleUser      Role = "user"
    RoleAssistant Role = "assistant"
)

type Message struct {
    Role    Role           `json:"role"`
    Content []ContentBlock `json:"content"`
}
```

**注意**：由于 `ContentBlock` 是 interface，JSON 序列化/反序列化需要自定义 `MarshalJSON`/`UnmarshalJSON`。

### 2.3 Tool Interface

```go
// Tool 是工具的抽象接口
type Tool interface {
    Name() string
    Description() string
    InputSchema() map[string]any
    IsReadOnly() bool
    Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error)
}

// ToolContext 是工具执行时的上下文
type ToolContext struct {
    WorkDir        string
    MessageHistory []Message
}

// ToolResult 是工具执行结果
type ToolResult struct {
    Content  string
    IsError  bool
}

// BaseTool 提供默认实现，可内嵌到具体工具中
type BaseTool struct{}

func (b BaseTool) IsReadOnly() bool { return false }
```

### 2.4 Event

```go
type EventType string

const (
    EventTurnStart     EventType = "turn_start"
    EventMessageStart  EventType = "message_start"
    EventMessageDelta  EventType = "message_delta"
    EventThinkingDelta EventType = "thinking_delta"
    EventMessageEnd    EventType = "message_end"
    EventToolUseStart  EventType = "tool_use_start"
    EventToolUseEnd    EventType = "tool_use_end"
    EventError         EventType = "error"
    EventComplete      EventType = "complete"
)

type Event struct {
    Type EventType              `json:"type"`
    Data map[string]any         `json:"data"`
}
```

### 2.5 Config

```go
type Config struct {
    BaseURL       string        // 默认: "https://api.anthropic.com"
    APIKey        string
    Model         string        // 默认: "claude-sonnet-4-6"
    WorkDir       string        // 默认: os.Getwd()
    MaxTokens     int           // 默认: 8192
    MaxTurns      int           // 默认: 100
    SystemPrompt  string        // 可选
    Timeout       time.Duration // 默认: 2m
    Stream        bool          // 默认: true
    HTTPClient    *http.Client  // 可选自定义
}

func DefaultConfig() Config {
    wd, _ := os.Getwd()
    return Config{
        BaseURL:   "https://api.anthropic.com",
        Model:     "claude-sonnet-4-6",
        WorkDir:   wd,
        MaxTokens: 8192,
        MaxTurns:  100,
        Timeout:   2 * time.Minute,
        Stream:    true,
    }
}
```

### 2.6 Agent

```go
type Agent struct {
    config          Config
    tools           map[string]Tool
    messageHistory  []Message
    turnCount       int
    client          *http.Client
    mu              sync.RWMutex
}

func NewAgent(cfg Config) *Agent

// 工具管理
func (a *Agent) RegisterTool(tool Tool)
func (a *Agent) UnregisterTool(name string)
func (a *Agent) ListTools() []Tool

// 核心方法
func (a *Agent) Run(ctx context.Context, input string) (<-chan Event, error)
func (a *Agent) Chat(ctx context.Context, messages []Message) (<-chan Event, error)

// 状态
func (a *Agent) MessageHistory() []Message
func (a *Agent) ClearHistory()
```

## 3. 预定义工具实现

### 3.1 ReadFileTool

```go
type ReadFileTool struct{ BaseTool }

func (r *ReadFileTool) Name() string        { return "read_file" }
func (r *ReadFileTool) Description() string { return "Read file contents from the working directory." }
func (r *ReadFileTool) IsReadOnly() bool    { return true }

func (r *ReadFileTool) InputSchema() map[string]any {
    return map[string]any{
        "type": "object",
        "properties": map[string]any{
            "file_path": map[string]any{"type": "string", "description": "Path to the file"},
            "offset":    map[string]any{"type": "integer", "description": "Line to start from"},
            "limit":     map[string]any{"type": "integer", "description": "Max lines to read"},
        },
        "required": []string{"file_path"},
    }
}

func (r *ReadFileTool) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
    filePath, ok := input["file_path"].(string)
    if !ok {
        return ToolResult{Content: "file_path is required", IsError: true}, nil
    }

    path := filepath.Join(toolCtx.WorkDir, filePath)
    content, err := os.ReadFile(path)
    if err != nil {
        return ToolResult{Content: fmt.Sprintf("Failed to read file: %v", err), IsError: true}, nil
    }

    return ToolResult{Content: string(content), IsError: false}, nil
}
```

### 3.2 WriteFileTool

```go
type WriteFileTool struct{ BaseTool }

func (w *WriteFileTool) Name() string        { return "write_file" }
func (w *WriteFileTool) Description() string { return "Write content to a file." }
func (w *WriteFileTool) IsReadOnly() bool    { return false }

func (w *WriteFileTool) InputSchema() map[string]any {
    return map[string]any{
        "type": "object",
        "properties": map[string]any{
            "file_path": map[string]any{"type": "string"},
            "content":   map[string]any{"type": "string"},
        },
        "required": []string{"file_path", "content"},
    }
}

func (w *WriteFileTool) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
    filePath := input["file_path"].(string)
    content := input["content"].(string)
    path := filepath.Join(toolCtx.WorkDir, filePath)

    if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
        return ToolResult{Content: fmt.Sprintf("Failed to create directory: %v", err), IsError: true}, nil
    }

    if err := os.WriteFile(path, []byte(content), 0644); err != nil {
        return ToolResult{Content: fmt.Sprintf("Failed to write file: %v", err), IsError: true}, nil
    }

    return ToolResult{Content: fmt.Sprintf("File written: %s", path), IsError: false}, nil
}
```

### 3.3 UpdateFileTool

```go
type UpdateFileTool struct{ BaseTool }

func (u *UpdateFileTool) Name() string        { return "update_file" }
func (u *UpdateFileTool) Description() string { return "Update a file by replacing old_string with new_string." }
func (u *UpdateFileTool) IsReadOnly() bool    { return false }

func (u *UpdateFileTool) InputSchema() map[string]any {
    return map[string]any{
        "type": "object",
        "properties": map[string]any{
            "file_path":   map[string]any{"type": "string"},
            "old_string":  map[string]any{"type": "string"},
            "new_string":  map[string]any{"type": "string"},
            "replace_all": map[string]any{"type": "boolean"},
        },
        "required": []string{"file_path", "old_string", "new_string"},
    }
}

func (u *UpdateFileTool) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
    filePath := input["file_path"].(string)
    oldStr := input["old_string"].(string)
    newStr := input["new_string"].(string)
    replaceAll, _ := input["replace_all"].(bool)

    path := filepath.Join(toolCtx.WorkDir, filePath)
    content, err := os.ReadFile(path)
    if err != nil {
        return ToolResult{Content: fmt.Sprintf("Failed to read: %v", err), IsError: true}, nil
    }

    var newContent string
    if replaceAll {
        newContent = strings.ReplaceAll(string(content), oldStr, newStr)
    } else {
        newContent = strings.Replace(string(content), oldStr, newStr, 1)
    }

    if newContent == string(content) {
        return ToolResult{Content: "old_string not found", IsError: true}, nil
    }

    if err := os.WriteFile(path, []byte(newContent), 0644); err != nil {
        return ToolResult{Content: fmt.Sprintf("Failed to write: %v", err), IsError: true}, nil
    }

    return ToolResult{Content: fmt.Sprintf("File updated: %s", path), IsError: false}, nil
}
```

### 3.4 BashTool

```go
type BashTool struct{ BaseTool }

func (b *BashTool) Name() string        { return "bash" }
func (b *BashTool) Description() string { return "Execute a shell command." }

func (b *BashTool) InputSchema() map[string]any {
    return map[string]any{
        "type": "object",
        "properties": map[string]any{
            "command":     map[string]any{"type": "string"},
            "description": map[string]any{"type": "string"},
            "timeout":     map[string]any{"type": "integer", "default": 120000},
        },
        "required": []string{"command"},
    }
}

func (b *BashTool) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
    command := input["command"].(string)
    timeoutMs := 120000
    if t, ok := input["timeout"].(float64); ok {
        timeoutMs = int(t)
    }

    ctx, cancel := context.WithTimeout(ctx, time.Duration(timeoutMs)*time.Millisecond)
    defer cancel()

    cmd := exec.CommandContext(ctx, "sh", "-c", command)
    cmd.Dir = toolCtx.WorkDir

    output, err := cmd.CombinedOutput()
    if ctx.Err() == context.DeadlineExceeded {
        return ToolResult{Content: "Command timed out", IsError: true}, nil
    }

    isError := err != nil
    return ToolResult{Content: string(output), IsError: isError}, nil
}
```

### 3.5 CurlTool

```go
type CurlTool struct {
    BaseTool
    client *http.Client
}

func (c *CurlTool) Name() string        { return "curl" }
func (c *CurlTool) Description() string { return "Make an HTTP request." }
func (c *CurlTool) IsReadOnly() bool    { return true }

func (c *CurlTool) InputSchema() map[string]any {
    return map[string]any{
        "type": "object",
        "properties": map[string]any{
            "url":     map[string]any{"type": "string"},
            "method":  map[string]any{"type": "string", "enum": []string{"GET", "POST", "PUT", "DELETE", "PATCH"}},
            "headers": map[string]any{"type": "object"},
            "body":    map[string]any{"type": "string"},
            "timeout": map[string]any{"type": "integer", "default": 30000},
        },
        "required": []string{"url"},
    }
}

func (c *CurlTool) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
    url := input["url"].(string)
    method := "GET"
    if m, ok := input["method"].(string); ok {
        method = m
    }

    var body io.Reader
    if b, ok := input["body"].(string); ok && b != "" {
        body = strings.NewReader(b)
    }

    req, err := http.NewRequestWithContext(ctx, method, url, body)
    if err != nil {
        return ToolResult{Content: fmt.Sprintf("Request error: %v", err), IsError: true}, nil
    }

    if headers, ok := input["headers"].(map[string]any); ok {
        for k, v := range headers {
            req.Header.Set(k, fmt.Sprintf("%v", v))
        }
    }

    resp, err := c.client.Do(req)
    if err != nil {
        return ToolResult{Content: fmt.Sprintf("HTTP error: %v", err), IsError: true}, nil
    }
    defer resp.Body.Close()

    respBody, _ := io.ReadAll(resp.Body)
    return ToolResult{Content: string(respBody), IsError: false}, nil
}
```

## 4. API 调用层

### 4.1 Client

```go
type APIClient struct {
    config     Config
    httpClient *http.Client
}

func NewAPIClient(cfg Config) *APIClient

// 发送流式请求
func (c *APIClient) StreamMessages(ctx context.Context, req APIRequest) (<-chan StreamEvent, error)

// 发送非流式请求
func (c *APIClient) SendMessages(ctx context.Context, req APIRequest) (*APIResponse, error)
```

### 4.2 请求/响应类型

```go
type APIRequest struct {
    Model      string              `json:"model"`
    Messages   []Message           `json:"messages"`
    System     string              `json:"system,omitempty"`
    Tools      []ToolDefinition    `json:"tools,omitempty"`
    MaxTokens  int                 `json:"max_tokens"`
    Stream     bool                `json:"stream,omitempty"`
}

type ToolDefinition struct {
    Name        string         `json:"name"`
    Description string         `json:"description"`
    InputSchema map[string]any `json:"input_schema"`
}

type APIResponse struct {
    ID           string   `json:"id"`
    Type         string   `json:"type"`
    Role         Role     `json:"role"`
    Content      []map[string]any `json:"content"`
    StopReason   *string  `json:"stop_reason"`
    Usage        Usage    `json:"usage"`
}

type Usage struct {
    InputTokens  int `json:"input_tokens"`
    OutputTokens int `json:"output_tokens"`
}
```

### 4.3 SSE 流式解析

```go
func (c *APIClient) StreamMessages(ctx context.Context, req APIRequest) (<-chan StreamEvent, error) {
    req.Stream = true
    body, _ := json.Marshal(req)

    httpReq, _ := http.NewRequestWithContext(ctx, "POST",
        c.config.BaseURL+"/v1/messages",
        bytes.NewReader(body))
    httpReq.Header.Set("Content-Type", "application/json")
    httpReq.Header.Set("x-api-key", c.config.APIKey)
    httpReq.Header.Set("anthropic-version", "2023-06-01")
    httpReq.Header.Set("Accept", "text/event-stream")

    resp, err := c.httpClient.Do(httpReq)
    if err != nil {
        return nil, err
    }

    events := make(chan StreamEvent, 10)
    go func() {
        defer close(events)
        defer resp.Body.Close()

        scanner := bufio.NewScanner(resp.Body)
        for scanner.Scan() {
            line := scanner.Text()
            if !strings.HasPrefix(line, "data: ") {
                continue
            }
            data := strings.TrimPrefix(line, "data: ")
            if data == "[DONE]" {
                break
            }

            var event StreamEvent
            if err := json.Unmarshal([]byte(data), &event); err == nil {
                select {
                case events <- event:
                case <-ctx.Done():
                    return
                }
            }
        }
    }()

    return events, nil
}
```

## 5. Agent Loop 实现

```go
func (a *Agent) Run(ctx context.Context, input string) (<-chan Event, error) {
    a.mu.Lock()
    defer a.mu.Unlock()

    // 初始用户消息
    a.messageHistory = append(a.messageHistory, Message{
        Role: RoleUser,
        Content: []ContentBlock{
            TextBlock{Text: input},
        },
    })

    events := make(chan Event, 100)
    go a.runLoop(ctx, events)

    return events, nil
}

func (a *Agent) runLoop(ctx context.Context, events chan<- Event) {
    defer close(events)

    systemPrompt := buildSystemPrompt(a.tools, a.config.SystemPrompt)

    for a.turnCount < a.config.MaxTurns {
        a.turnCount++
        events <- Event{Type: EventTurnStart, Data: map[string]any{"turn": a.turnCount}}

        // 构建请求
        toolDefs := make([]ToolDefinition, 0, len(a.tools))
        for _, tool := range a.tools {
            toolDefs = append(toolDefs, ToolDefinition{
                Name:        tool.Name(),
                Description: tool.Description(),
                InputSchema: tool.InputSchema(),
            })
        }

        req := APIRequest{
            Model:     a.config.Model,
            Messages:  a.messageHistory,
            System:    systemPrompt,
            Tools:     toolDefs,
            MaxTokens: a.config.MaxTokens,
            Stream:    a.config.Stream,
        }

        client := NewAPIClient(a.config)
        stream, err := client.StreamMessages(ctx, req)
        if err != nil {
            events <- Event{Type: EventError, Data: map[string]any{"message": err.Error()}}
            return
        }

        // 流式解析
        var assistantContent []ContentBlock
        events <- Event{Type: EventMessageStart}

        for event := range stream {
            // 解析 event，累积 assistant content
            // ... 根据 event.Type 处理
        }
        events <- Event{Type: EventMessageEnd}

        // 将 assistant 消息加入历史
        a.messageHistory = append(a.messageHistory, Message{
            Role:    RoleAssistant,
            Content: assistantContent,
        })

        // 提取 tool_use
        var toolUses []ToolUseBlock
        for _, block := range assistantContent {
            if tu, ok := block.(ToolUseBlock); ok {
                toolUses = append(toolUses, tu)
            }
        }

        if len(toolUses) == 0 {
            // 任务完成
            finalText := extractLastText(assistantContent)
            events <- Event{Type: EventComplete, Data: map[string]any{"final_content": finalText}}
            return
        }

        // 执行工具
        results := a.executeTools(ctx, toolUses, events)

        // 将 tool_result 加入历史
        var resultBlocks []ContentBlock
        for _, r := range results {
            resultBlocks = append(resultBlocks, r)
        }
        a.messageHistory = append(a.messageHistory, Message{
            Role:    RoleUser,
            Content: resultBlocks,
        })
    }

    events <- Event{Type: EventError, Data: map[string]any{"message": "max turns reached"}}
}

func (a *Agent) executeTools(ctx context.Context, toolUses []ToolUseBlock, events chan<- Event) []ToolResultBlock {
    // 分组：只读并发，写入串行
    var readOnly []ToolUseBlock
    var write []ToolUseBlock

    for _, tu := range toolUses {
        if tool, ok := a.tools[tu.Name]; ok && tool.IsReadOnly() {
            readOnly = append(readOnly, tu)
        } else {
            write = append(write, tu)
        }
    }

    var results []ToolResultBlock
    var mu sync.Mutex
    var wg sync.WaitGroup

    // 并发执行只读工具
    for _, tu := range readOnly {
        wg.Add(1)
        go func(tu ToolUseBlock) {
            defer wg.Done()
            result := a.executeSingleTool(ctx, tu, events)
            mu.Lock()
            results = append(results, result)
            mu.Unlock()
        }(tu)
    }
    wg.Wait()

    // 串行执行写工具
    for _, tu := range write {
        result := a.executeSingleTool(ctx, tu, events)
        results = append(results, result)
    }

    return results
}

func (a *Agent) executeSingleTool(ctx context.Context, tu ToolUseBlock, events chan<- Event) ToolResultBlock {
    events <- Event{Type: EventToolUseStart, Data: map[string]any{
        "name": tu.Name, "id": tu.ID,
    }}

    tool, ok := a.tools[tu.Name]
    if !ok {
        return ToolResultBlock{
            ToolUseID: tu.ID,
            Content:   fmt.Sprintf("Tool not found: %s", tu.Name),
            IsError:   boolPtr(true),
        }
    }

    var input map[string]any
    json.Unmarshal(tu.Input, &input)

    toolCtx := ToolContext{
        WorkDir:        a.config.WorkDir,
        MessageHistory: a.messageHistory,
    }

    result, err := tool.Call(ctx, input, toolCtx)
    if err != nil {
        result = ToolResult{Content: err.Error(), IsError: true}
    }

    events <- Event{Type: EventToolUseEnd, Data: map[string]any{
        "name": tu.Name, "id": tu.ID, "result": result,
    }}

    return ToolResultBlock{
        ToolUseID: tu.ID,
        Content:   result.Content,
        IsError:   boolPtr(result.IsError),
    }
}
```

## 6. 自定义工具注册

```go
// 定义自定义工具
type MyTool struct{ BaseTool }

func (m *MyTool) Name() string        { return "my_tool" }
func (m *MyTool) Description() string { return "Does something custom" }
func (m *MyTool) InputSchema() map[string]any {
    return map[string]any{
        "type": "object",
        "properties": map[string]any{
            "param1": map[string]any{"type": "string"},
        },
        "required": []string{"param1"},
    }
}

func (m *MyTool) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
    param1 := input["param1"].(string)
    return ToolResult{
        Content: fmt.Sprintf("Processed: %s", param1),
        IsError: false,
    }, nil
}

// 使用
agent := agentlib.NewAgent(agentlib.DefaultConfig())
agent.RegisterTool(&MyTool{})
```

## 7. 目录结构

```
go/
├── go.mod
├── agent.go              # Agent 核心
├── config.go             # Config
├── types.go              # Message, ContentBlock, Event 等类型
├── tool.go               # Tool interface, ToolContext, ToolResult
├── tools/
│   ├── read_file.go
│   ├── write_file.go
│   ├── update_file.go
│   ├── bash.go
│   └── curl.go
├── client.go             # Anthropic API 客户端
├── stream.go             # SSE 流式解析
├── prompt.go             # 系统提示词
└── errors.go             # 错误类型

// 测试
go/
└── ...
```

## 8. 依赖

```go
// go.mod
module github.com/steven0lisa/agent-libs/go

go 1.21

require (
    // 标准库足够，无外部依赖
)
```

Go 版本使用标准库的 `net/http`、`context`、`encoding/json`、`os/exec` 等即可实现全部功能，无需第三方依赖。
