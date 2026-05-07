# PRD - Java 实现

## 1. 语言特性映射

| 通用概念 | Java 实现 |
|----------|-----------|
| Agent | `class Agent` |
| Tool | `interface Tool` |
| Message | `class Message` + `enum Role` |
| ContentBlock | `sealed interface ContentBlock` + `record` 实现 |
| Event | `sealed interface Event` + `record` 实现 |
| Config | `record AgentConfig` |
| 异步 | `CompletableFuture` + `java.util.concurrent.Flow` |
| 事件流 | `Flow.Publisher<Event>` 或 `BlockingQueue<Event>` |
| 错误处理 | 异常（`AgentException` extends `RuntimeException`） |
| JSON | `com.fasterxml.jackson.databind.JsonNode` |
| HTTP | `java.net.http.HttpClient` (Java 11+) |

**最低 Java 版本**: Java 17（使用 sealed interface 和 record）

## 2. 核心类型设计

### 2.1 ContentBlock

```java
public sealed interface ContentBlock {
    String type();
}

public record TextBlock(String text) implements ContentBlock {
    @Override public String type() { return "text"; }
}

public record ToolUseBlock(
    String name,
    String id,
    JsonNode input
) implements ContentBlock {
    @Override public String type() { return "tool_use"; }
}

public record ToolResultBlock(
    String toolUseId,
    String content,
    Boolean isError
) implements ContentBlock {
    @Override public String type() { return "tool_result"; }
}

public record ThinkingBlock(
    String thinking,
    String signature
) implements ContentBlock {
    @Override public String type() { return "thinking"; }
}
```

### 2.2 Message

```java
public enum Role {
    USER("user"),
    ASSISTANT("assistant");

    private final String value;
    Role(String value) { this.value = value; }
    public String value() { return value; }
}

public record Message(Role role, List<ContentBlock> content) {
    public Message {
        content = List.copyOf(content);
    }

    // 便捷构造器
    public static Message user(String text) {
        return new Message(Role.USER, List.of(new TextBlock(text)));
    }
}
```

### 2.3 Tool Interface

```java
public interface Tool {
    String name();
    String description();
    JsonNode inputSchema();

    default boolean isReadOnly() {
        return false;
    }

    ToolResult call(Map<String, Object> input, ToolContext context);
}

public record ToolContext(
    Path workDir,
    List<Message> messageHistory
) {}

public record ToolResult(
    String content,
    boolean isError
) {
    public static ToolResult success(String content) {
        return new ToolResult(content, false);
    }
    public static ToolResult error(String content) {
        return new ToolResult(content, true);
    }
}
```

### 2.4 Event

```java
public sealed interface Event {
    String type();
}

public record TurnStartEvent(int turn) implements Event {
    @Override public String type() { return "turn_start"; }
}

public record MessageStartEvent() implements Event {
    @Override public String type() { return "message_start"; }
}

public record MessageDeltaEvent(String text) implements Event {
    @Override public String type() { return "message_delta"; }
}

public record ThinkingDeltaEvent(String thinking) implements Event {
    @Override public String type() { return "thinking_delta"; }
}

public record MessageEndEvent() implements Event {
    @Override public String type() { return "message_end"; }
}

public record ToolUseStartEvent(String name, String id, JsonNode input) implements Event {
    @Override public String type() { return "tool_use_start"; }
}

public record ToolUseEndEvent(String name, String id, ToolResult result) implements Event {
    @Override public String type() { return "tool_use_end"; }
}

public record ErrorEvent(String message) implements Event {
    @Override public String type() { return "error"; }
}

public record CompleteEvent(String finalContent) implements Event {
    @Override public String type() { return "complete"; }
}
```

### 2.5 AgentConfig

```java
public record AgentConfig(
    String baseUrl,       // 默认: "https://api.anthropic.com"
    String apiKey,
    String model,         // 默认: "claude-sonnet-4-6"
    Path workDir,         // 默认: Paths.get("").toAbsolutePath()
    int maxTokens,        // 默认: 8192
    int maxTurns,         // 默认: 100
    String systemPrompt,  // 可选
    Duration timeout,     // 默认: Duration.ofMinutes(2)
    boolean stream,       // 默认: true
    List<Tool> customTools // 初始自定义工具
) {
    public AgentConfig {
        if (baseUrl == null) baseUrl = "https://api.anthropic.com";
        if (model == null) model = "claude-sonnet-4-6";
        if (workDir == null) workDir = Paths.get("").toAbsolutePath();
        if (maxTokens <= 0) maxTokens = 8192;
        if (maxTurns <= 0) maxTurns = 100;
        if (timeout == null) timeout = Duration.ofMinutes(2);
        if (customTools == null) customTools = List.of();
    }

    public static AgentConfigBuilder builder() {
        return new AgentConfigBuilder();
    }
}
```

### 2.6 Agent

```java
public class Agent {
    private final AgentConfig config;
    private final HttpClient httpClient;
    private final Map<String, Tool> tools;
    private final List<Message> messageHistory;
    private int turnCount;

    public Agent(AgentConfig config) { ... }

    // 工具管理
    public void registerTool(Tool tool);
    public void unregisterTool(String name);
    public List<Tool> listTools();

    // 核心方法
    public Flow.Publisher<Event> run(String input);
    public CompletableFuture<String> runAsync(String input);
    public Flow.Publisher<Event> chat(List<Message> messages);

    // 状态
    public List<Message> getMessageHistory();
    public void clearHistory();
}
```

## 3. 预定义工具实现

### 3.1 ReadFileTool

```java
public class ReadFileTool implements Tool {
    @Override
    public String name() { return "read_file"; }

    @Override
    public String description() {
        return "Read file contents from the working directory.";
    }

    @Override
    public boolean isReadOnly() { return true; }

    @Override
    public JsonNode inputSchema() {
        return JsonNodeFactory.instance.objectNode()
            .put("type", "object")
            .set("properties", JsonNodeFactory.instance.objectNode()
                .set("file_path", JsonNodeFactory.instance.objectNode()
                    .put("type", "string")
                    .put("description", "Path to the file"))
                .set("offset", JsonNodeFactory.instance.objectNode()
                    .put("type", "integer"))
                .set("limit", JsonNodeFactory.instance.objectNode()
                    .put("type", "integer")))
            .set("required", JsonNodeFactory.instance.arrayNode().add("file_path"));
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String filePath = (String) input.get("file_path");
        Path path = context.workDir().resolve(filePath).normalize();

        try {
            String content = Files.readString(path);
            return ToolResult.success(content);
        } catch (IOException e) {
            return ToolResult.error("Failed to read file: " + e.getMessage());
        }
    }
}
```

### 3.2 WriteFileTool

```java
public class WriteFileTool implements Tool {
    @Override
    public String name() { return "write_file"; }

    @Override
    public String description() {
        return "Write content to a file. Creates if not exists, overwrites if exists.";
    }

    @Override
    public boolean isReadOnly() { return false; }

    @Override
    public JsonNode inputSchema() {
        return JsonNodeFactory.instance.objectNode()
            .put("type", "object")
            .set("properties", JsonNodeFactory.instance.objectNode()
                .set("file_path", JsonNodeFactory.instance.objectNode().put("type", "string"))
                .set("content", JsonNodeFactory.instance.objectNode().put("type", "string")))
            .set("required", JsonNodeFactory.instance.arrayNode().add("file_path").add("content"));
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String filePath = (String) input.get("file_path");
        String content = (String) input.get("content");
        Path path = context.workDir().resolve(filePath).normalize();

        try {
            Files.createDirectories(path.getParent());
            Files.writeString(path, content);
            return ToolResult.success("File written: " + path);
        } catch (IOException e) {
            return ToolResult.error("Failed to write file: " + e.getMessage());
        }
    }
}
```

### 3.3 UpdateFileTool

```java
public class UpdateFileTool implements Tool {
    @Override
    public String name() { return "update_file"; }

    @Override
    public String description() {
        return "Update a file by replacing old_string with new_string.";
    }

    @Override
    public boolean isReadOnly() { return false; }

    @Override
    public JsonNode inputSchema() {
        return JsonNodeFactory.instance.objectNode()
            .put("type", "object")
            .set("properties", JsonNodeFactory.instance.objectNode()
                .set("file_path", JsonNodeFactory.instance.objectNode().put("type", "string"))
                .set("old_string", JsonNodeFactory.instance.objectNode().put("type", "string"))
                .set("new_string", JsonNodeFactory.instance.objectNode().put("type", "string"))
                .set("replace_all", JsonNodeFactory.instance.objectNode().put("type", "boolean")))
            .set("required", JsonNodeFactory.instance.arrayNode()
                .add("file_path").add("old_string").add("new_string"));
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String filePath = (String) input.get("file_path");
        String oldStr = (String) input.get("old_string");
        String newStr = (String) input.get("new_string");
        boolean replaceAll = Boolean.TRUE.equals(input.get("replace_all"));

        Path path = context.workDir().resolve(filePath).normalize();

        try {
            String content = Files.readString(path);
            String newContent = replaceAll
                ? content.replace(oldStr, newStr)
                : content.replaceFirst(Pattern.quote(oldStr), Matcher.quoteReplacement(newStr));

            if (newContent.equals(content)) {
                return ToolResult.error("old_string not found in file");
            }

            Files.writeString(path, newContent);
            return ToolResult.success("File updated: " + path);
        } catch (IOException e) {
            return ToolResult.error("Failed: " + e.getMessage());
        }
    }
}
```

### 3.4 BashTool

```java
public class BashTool implements Tool {
    @Override
    public String name() { return "bash"; }

    @Override
    public String description() {
        return "Execute a shell command in the working directory.";
    }

    @Override
    public JsonNode inputSchema() {
        return JsonNodeFactory.instance.objectNode()
            .put("type", "object")
            .set("properties", JsonNodeFactory.instance.objectNode()
                .set("command", JsonNodeFactory.instance.objectNode()
                    .put("type", "string")
                    .put("description", "The shell command"))
                .set("description", JsonNodeFactory.instance.objectNode()
                    .put("type", "string"))
                .set("timeout", JsonNodeFactory.instance.objectNode()
                    .put("type", "integer")
                    .put("default", 120000)))
            .set("required", JsonNodeFactory.instance.arrayNode().add("command"));
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String command = (String) input.get("command");
        int timeoutMs = 120000;
        if (input.get("timeout") instanceof Number n) {
            timeoutMs = n.intValue();
        }

        try {
            ProcessBuilder pb = new ProcessBuilder("sh", "-c", command)
                .directory(context.workDir().toFile())
                .redirectErrorStream(true);

            Process process = pb.start();
            boolean finished = process.waitFor(timeoutMs, TimeUnit.MILLISECONDS);

            if (!finished) {
                process.destroyForcibly();
                return ToolResult.error("Command timed out after " + timeoutMs + "ms");
            }

            String output = new String(process.getInputStream().readAllBytes(), StandardCharsets.UTF_8);
            return new ToolResult(output, process.exitValue() != 0);
        } catch (Exception e) {
            return ToolResult.error("Failed to execute: " + e.getMessage());
        }
    }
}
```

### 3.5 CurlTool

```java
public class CurlTool implements Tool {
    private final HttpClient httpClient;

    public CurlTool(HttpClient httpClient) {
        this.httpClient = httpClient;
    }

    @Override
    public String name() { return "curl"; }

    @Override
    public String description() { return "Make an HTTP request."; }

    @Override
    public boolean isReadOnly() { return true; }

    @Override
    public JsonNode inputSchema() {
        return JsonNodeFactory.instance.objectNode()
            .put("type", "object")
            .set("properties", JsonNodeFactory.instance.objectNode()
                .set("url", JsonNodeFactory.instance.objectNode().put("type", "string"))
                .set("method", JsonNodeFactory.instance.objectNode()
                    .put("type", "string")
                    .put("enum", "GET,POST,PUT,DELETE,PATCH"))
                .set("headers", JsonNodeFactory.instance.objectNode().put("type", "object"))
                .set("body", JsonNodeFactory.instance.objectNode().put("type", "string"))
                .set("timeout", JsonNodeFactory.instance.objectNode()
                    .put("type", "integer").put("default", 30000)))
            .set("required", JsonNodeFactory.instance.arrayNode().add("url"));
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String url = (String) input.get("url");
        String method = (String) input.getOrDefault("method", "GET");
        String body = (String) input.get("body");

        try {
            HttpRequest.Builder builder = HttpRequest.newBuilder()
                .uri(URI.create(url))
                .timeout(Duration.ofSeconds(30));

            HttpRequest.BodyPublisher bodyPublisher = body != null
                ? HttpRequest.BodyPublishers.ofString(body)
                : HttpRequest.BodyPublishers.noBody();

            builder.method(method, bodyPublisher);

            if (input.get("headers") instanceof Map<?, ?> headers) {
                headers.forEach((k, v) -> builder.header((String) k, String.valueOf(v)));
            }

            HttpResponse<String> response = httpClient.send(
                builder.build(), HttpResponse.BodyHandlers.ofString());

            return ToolResult.success(response.body());
        } catch (Exception e) {
            return ToolResult.error("HTTP error: " + e.getMessage());
        }
    }
}
```

## 4. API 调用层

### 4.1 AnthropicClient

```java
public class AnthropicClient {
    private final AgentConfig config;
    private final HttpClient httpClient;
    private final ObjectMapper objectMapper;

    public AnthropicClient(AgentConfig config) {
        this.config = config;
        this.httpClient = HttpClient.newBuilder()
            .connectTimeout(config.timeout())
            .build();
        this.objectMapper = new ObjectMapper();
    }

    public Flow.Publisher<StreamEvent> streamMessages(
        List<Message> messages,
        String systemPrompt,
        List<ToolDefinition> tools
    ) { ... }

    public Message sendMessages(
        List<Message> messages,
        String systemPrompt,
        List<ToolDefinition> tools
    ) { ... }
}
```

### 4.2 SSE 流式解析

```java
public Flow.Publisher<StreamEvent> streamMessages(...) {
    return subscriber -> {
        try {
            // 构建请求
            ObjectNode requestBody = objectMapper.createObjectNode();
            requestBody.put("model", config.model());
            requestBody.put("max_tokens", config.maxTokens());
            requestBody.put("stream", true);
            requestBody.set("messages", objectMapper.valueToTree(messages));
            requestBody.put("system", systemPrompt);
            requestBody.set("tools", objectMapper.valueToTree(tools));

            HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(config.baseUrl() + "/v1/messages"))
                .header("Content-Type", "application/json")
                .header("x-api-key", config.apiKey())
                .header("anthropic-version", "2023-06-01")
                .header("Accept", "text/event-stream")
                .POST(HttpRequest.BodyPublishers.ofString(requestBody.toString()))
                .build();

            httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofLines())
                .thenAccept(response -> {
                    response.body().forEach(line -> {
                        if (line.startsWith("data: ")) {
                            String data = line.substring(6);
                            if ("[DONE]".equals(data)) return;

                            try {
                                StreamEvent event = parseStreamEvent(data);
                                subscriber.onNext(event);
                            } catch (Exception ignored) {}
                        }
                    });
                    subscriber.onComplete();
                })
                .exceptionally(ex -> {
                    subscriber.onError(ex);
                    return null;
                });
        } catch (Exception e) {
            subscriber.onError(e);
        }
    };
}
```

## 5. Agent Loop 实现

```java
public Flow.Publisher<Event> run(String input) {
    return subscriber -> {
        // 初始用户消息
        messageHistory.add(Message.user(input));

        executor.submit(() -> {
            try {
                runLoop(subscriber);
            } catch (Exception e) {
                subscriber.onNext(new ErrorEvent(e.getMessage()));
            } finally {
                subscriber.onComplete();
            }
        });
    };
}

private void runLoop(Flow.Subscriber<? super Event> subscriber) {
    String systemPrompt = PromptBuilder.build(tools.values(), config.systemPrompt());

    while (turnCount < config.maxTurns()) {
        turnCount++;
        subscriber.onNext(new TurnStartEvent(turnCount));

        // 构建工具定义
        List<ToolDefinition> toolDefs = tools.values().stream()
            .map(t -> new ToolDefinition(t.name(), t.description(), t.inputSchema()))
            .toList();

        // 调用 API
        var streamEvents = client.streamMessages(messageHistory, systemPrompt, toolDefs);

        // 流式接收
        subscriber.onNext(new MessageStartEvent());
        List<ContentBlock> assistantContent = new ArrayList<>();

        // 解析流事件，累积内容...
        // (实际实现需要使用 CountDownLatch 或类似的同步机制等待流结束)

        subscriber.onNext(new MessageEndEvent());

        // 将 assistant 消息加入历史
        messageHistory.add(new Message(Role.ASSISTANT, assistantContent));

        // 提取 tool_use
        List<ToolUseBlock> toolUses = assistantContent.stream()
            .filter(b -> b instanceof ToolUseBlock)
            .map(b -> (ToolUseBlock) b)
            .toList();

        if (toolUses.isEmpty()) {
            String finalText = extractText(assistantContent);
            subscriber.onNext(new CompleteEvent(finalText));
            return;
        }

        // 执行工具
        List<ToolResultBlock> results = executeTools(toolUses, subscriber);

        // 将 tool_result 加入历史
        messageHistory.add(new Message(Role.USER,
            results.stream().map(r -> (ContentBlock) r).toList()));
    }

    subscriber.onNext(new ErrorEvent("Max turns reached"));
}

private List<ToolResultBlock> executeTools(
    List<ToolUseBlock> toolUses,
    Flow.Subscriber<? super Event> subscriber
) {
    // 分组
    List<ToolUseBlock> readOnly = new ArrayList<>();
    List<ToolUseBlock> write = new ArrayList<>();

    for (ToolUseBlock tu : toolUses) {
        Tool tool = tools.get(tu.name());
        if (tool != null && tool.isReadOnly()) {
            readOnly.add(tu);
        } else {
            write.add(tu);
        }
    }

    List<ToolResultBlock> results = Collections.synchronizedList(new ArrayList<>());

    // 并发执行只读工具
    List<CompletableFuture<Void>> futures = readOnly.stream()
        .map(tu -> CompletableFuture.runAsync(() -> {
            ToolResultBlock result = executeSingleTool(tu, subscriber);
            results.add(result);
        }))
        .toList();
    CompletableFuture.allOf(futures.toArray(new CompletableFuture[0])).join();

    // 串行执行写工具
    for (ToolUseBlock tu : write) {
        results.add(executeSingleTool(tu, subscriber));
    }

    return results;
}

private ToolResultBlock executeSingleTool(
    ToolUseBlock tu,
    Flow.Subscriber<? super Event> subscriber
) {
    subscriber.onNext(new ToolUseStartEvent(tu.name(), tu.id(), tu.input()));

    Tool tool = tools.get(tu.name());
    if (tool == null) {
        return new ToolResultBlock(tu.id(),
            "Tool not found: " + tu.name(), true);
    }

    Map<String, Object> input = objectMapper.convertValue(tu.input(), new TypeReference<>() {});
    ToolContext ctx = new ToolContext(config.workDir(), List.copyOf(messageHistory));

    ToolResult result = tool.call(input, ctx);

    subscriber.onNext(new ToolUseEndEvent(tu.name(), tu.id(), result));

    return new ToolResultBlock(tu.id(), result.content(), result.isError());
}
```

## 6. 自定义工具注册

```java
// 定义自定义工具
public class MyTool implements Tool {
    @Override
    public String name() { return "my_tool"; }

    @Override
    public String description() { return "Does something custom"; }

    @Override
    public JsonNode inputSchema() {
        return JsonNodeFactory.instance.objectNode()
            .put("type", "object")
            .set("properties", JsonNodeFactory.instance.objectNode()
                .set("param1", JsonNodeFactory.instance.objectNode().put("type", "string")))
            .set("required", JsonNodeFactory.instance.arrayNode().add("param1"));
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String param1 = (String) input.get("param1");
        return ToolResult.success("Processed: " + param1);
    }
}

// 使用
AgentConfig config = AgentConfig.builder()
    .apiKey("sk-...")
    .model("claude-sonnet-4-6")
    .build();

Agent agent = new Agent(config);
agent.registerTool(new MyTool());

agent.run("Hello").subscribe(new Flow.Subscriber<>() {
    @Override public void onSubscribe(Flow.Subscription s) { s.request(Long.MAX_VALUE); }
    @Override public void onNext(Event event) { System.out.println(event); }
    @Override public void onError(Throwable t) { t.printStackTrace(); }
    @Override public void onComplete() { System.out.println("Done"); }
});
```

## 7. 目录结构

```
java/
├── pom.xml
└── src/
    ├── main/java/com/agentlib/
    │   ├── Agent.java
    │   ├── AgentConfig.java
    │   ├── Message.java
    │   ├── Role.java
    │   ├── ContentBlock.java
    │   ├── Event.java
    │   ├── Tool.java
    │   ├── ToolContext.java
    │   ├── ToolResult.java
    │   ├── AnthropicClient.java
    │   ├── PromptBuilder.java
    │   ├── StreamEvent.java
    │   ├── ToolDefinition.java
    │   ├── AgentException.java
    │   └── tools/
    │       ├── ReadFileTool.java
    │       ├── WriteFileTool.java
    │       ├── UpdateFileTool.java
    │       ├── BashTool.java
    │       └── CurlTool.java
    └── test/java/com/agentlib/
        └── ...
```

## 8. 依赖 (pom.xml)

```xml
<dependencies>
    <!-- Jackson for JSON -->
    <dependency>
        <groupId>com.fasterxml.jackson.core</groupId>
        <artifactId>jackson-databind</artifactId>
        <version>2.17.0</version>
    </dependency>

    <!-- JUnit 5 for testing -->
    <dependency>
        <groupId>org.junit.jupiter</groupId>
        <artifactId>junit-jupiter</artifactId>
        <version>5.10.0</version>
        <scope>test</scope>
    </dependency>

    <!-- Mockito for mocking -->
    <dependency>
        <groupId>org.mockito</groupId>
        <artifactId>mockito-core</artifactId>
        <version>5.11.0</version>
        <scope>test</scope>
    </dependency>
</dependencies>
```
