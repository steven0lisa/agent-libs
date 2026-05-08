package com.agentlib;

import com.agentlib.skills.SkillInfo;
import com.agentlib.skills.SkillLoader;
import com.agentlib.skills.SkillTool;
import com.agentlib.tools.BashTool;
import com.agentlib.tools.CurlTool;
import com.agentlib.tools.GlobTool;
import com.agentlib.tools.GrepTool;
import com.agentlib.tools.ReadFileTool;
import com.agentlib.tools.SubAgentTool;
import com.agentlib.tools.UpdateFileTool;
import com.agentlib.tools.WriteFileTool;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.net.http.HttpClient;
import java.time.Duration;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Flow;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.Consumer;

/**
 * Agent with lifecycle management (run/pause/resume/stop).
 * Emits events via Flow.Publisher for each step of execution.
 */
public class Agent {
    private final AgentConfig config;
    private final Map<String, Tool> tools;
    private final List<Message> messageHistory;
    private int turnCount;
    private long startTime;
    private final ObjectMapper objectMapper;
    private final ExecutorService executor;
    private SkillLoader skillLoader;

    // Lifecycle state
    private volatile AgentState state;
    private final Object stateLock = new Object();
    private final AtomicBoolean pauseFlag;
    private final AtomicBoolean stopFlag;

    // Callback
    private Consumer<Event> callback;

    /**
     * Agent lifecycle states.
     */
    public enum AgentState {
        IDLE, RUNNING, PAUSED, STOPPING, TERMINATED, COMPLETED
    }

    public Agent(AgentConfig config) {
        this.config = config;
        this.tools = new ConcurrentHashMap<>();
        this.messageHistory = Collections.synchronizedList(new ArrayList<>());
        this.turnCount = 0;
        this.objectMapper = new ObjectMapper();
        this.executor = Executors.newCachedThreadPool();
        this.state = AgentState.IDLE;
        this.pauseFlag = new AtomicBoolean(true); // not paused
        this.stopFlag = new AtomicBoolean(false);

        // Register built-in tools
        registerTool(new ReadFileTool());
        registerTool(new WriteFileTool());
        registerTool(new UpdateFileTool());
        registerTool(new GrepTool());
        registerTool(new GlobTool());
        registerTool(new BashTool(config.bashWhitelist(), config.bashBlacklist()));
        registerTool(new CurlTool(
            HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(30))
                .followRedirects(HttpClient.Redirect.NORMAL)
                .build(),
            config.curlWhitelist(),
            config.curlBlacklist()
        ));

        // Register custom tools
        for (Tool tool : config.customTools()) {
            registerTool(tool);
        }

        // Register subagent tool if enabled
        if (config.enableSubagent()) {
            registerTool(new SubAgentTool(config, messageHistory, tools));
        }

        // Initialize skills system if enabled
        if (config.enableSkills()) {
            this.skillLoader = new SkillLoader(
                config.skillsDir(),
                config.includeProjectSkills(),
                config.skillsProjectDir()
            );
            registerTool(new SkillTool(skillLoader));
        }
    }

    // ------------------------------------------------------------------
    // State management
    // ------------------------------------------------------------------

    public AgentState getState() {
        return state;
    }

    /**
     * Pause the agent. The current turn will finish, then pause.
     */
    public void pause() {
        synchronized (stateLock) {
            if (state == AgentState.RUNNING) {
                state = AgentState.PAUSED;
                pauseFlag.set(false);
            }
        }
    }

    /**
     * Resume a paused agent.
     */
    public void resume() {
        synchronized (stateLock) {
            if (state == AgentState.PAUSED) {
                state = AgentState.RUNNING;
                pauseFlag.set(true);
                stateLock.notifyAll();
            }
        }
    }

    /**
     * Stop the agent. Cannot be resumed.
     */
    public void stop() {
        synchronized (stateLock) {
            state = AgentState.STOPPING;
            stopFlag.set(true);
            pauseFlag.set(true); // unblock if paused
            stateLock.notifyAll();
        }
    }

    private boolean checkState() {
        AgentState current = state;
        return current != AgentState.STOPPING && current != AgentState.TERMINATED;
    }

    private void waitIfPaused() throws InterruptedException {
        synchronized (stateLock) {
            while (state == AgentState.PAUSED && !stopFlag.get()) {
                stateLock.wait(100);
            }
        }
    }

    // ------------------------------------------------------------------
    // Callback
    // ------------------------------------------------------------------

    public void setCallback(Consumer<Event> callback) {
        this.callback = callback;
    }

    private void emitEvent(Event event) {
        if (callback != null) {
            try {
                callback.accept(event);
            } catch (Exception e) {
                // Callback errors should not break the agent
            }
        }
    }

    // ------------------------------------------------------------------
    // Tool management
    // ------------------------------------------------------------------

    public void registerTool(Tool tool) {
        tools.put(tool.name(), tool);
    }

    public void unregisterTool(String name) {
        tools.remove(name);
    }

    public List<Tool> listTools() {
        return List.copyOf(tools.values());
    }

    // ------------------------------------------------------------------
    // History
    // ------------------------------------------------------------------

    public List<Message> getMessageHistory() {
        return messageHistory;
    }

    public void clearHistory() {
        messageHistory.clear();
        turnCount = 0;
    }

    // ------------------------------------------------------------------
    // Core methods
    // ------------------------------------------------------------------

    /**
     * Run the agent with the given input.
     * Returns a Flow.Publisher that emits events during execution.
     */
    public Flow.Publisher<Event> run(String input) {
        messageHistory.add(Message.user(input));
        startTime = System.currentTimeMillis();
        return runLoop();
    }

    /**
     * Continue the conversation with existing messages.
     */
    public Flow.Publisher<Event> chat(List<Message> messages) {
        messageHistory.addAll(messages);
        startTime = System.currentTimeMillis();
        return runLoop();
    }

    /**
     * Run the agent asynchronously and return the final content.
     */
    public CompletableFuture<String> runAsync(String input) {
        CompletableFuture<String> future = new CompletableFuture<>();
        Flow.Publisher<Event> publisher = run(input);

        publisher.subscribe(new Flow.Subscriber<>() {
            private Flow.Subscription subscription;
            private String finalContent = "";

            @Override
            public void onSubscribe(Flow.Subscription subscription) {
                this.subscription = subscription;
                subscription.request(Long.MAX_VALUE);
            }

            @Override
            public void onNext(Event event) {
                if (event instanceof CompleteEvent ce) {
                    finalContent = ce.finalContent();
                } else if (event instanceof ErrorEvent ee) {
                    future.completeExceptionally(new AgentException(ee.message()));
                }
            }

            @Override
            public void onError(Throwable throwable) {
                future.completeExceptionally(throwable);
            }

            @Override
            public void onComplete() {
                if (!future.isDone()) {
                    future.complete(finalContent);
                }
            }
        });

        return future;
    }

    // ------------------------------------------------------------------
    // Agent loop
    // ------------------------------------------------------------------

    private Flow.Publisher<Event> runLoop() {
        return subscriber -> {
            executor.submit(() -> {
                try {
                    synchronized (stateLock) {
                        state = AgentState.RUNNING;
                        startTime = System.currentTimeMillis();
                        stopFlag.set(false);
                        pauseFlag.set(true);
                    }

                    // Load skills for prompt
                    List<SkillInfo> skills = List.of();
                    if (skillLoader != null) {
                        skills = skillLoader.discoverAll();
                    }

                    String systemPrompt = PromptBuilder.build(
                        tools.values(),
                        config.systemPrompt(),
                        config.enableSubagent(),
                        config.subagentMaxTurns(),
                        skills
                    );

                    AnthropicClient client = new AnthropicClient(config);
                    int consecutiveCompactFailures = 0;

                    while (turnCount < config.maxTurns()) {
                        // Check stop
                        if (!checkState()) {
                            break;
                        }

                        // Wait if paused
                        waitIfPaused();
                        if (!checkState()) {
                            break;
                        }

                        turnCount++;
                        TurnStartEvent turnEvent = new TurnStartEvent(turnCount);
                        subscriber.onNext(turnEvent);
                        emitEvent(turnEvent);

                        // Check duration limit
                        if (config.maxDurationMs() > 0 && startTime > 0) {
                            long elapsed = System.currentTimeMillis() - startTime;
                            if (elapsed > config.maxDurationMs()) {
                                ErrorEvent error = new ErrorEvent("Max duration exceeded");
                                subscriber.onNext(error);
                                emitEvent(error);
                                break;
                            }
                        }

                        // Auto compact check
                        if (config.autoCompact()) {
                            AutoCompact.CompactResult result = AutoCompact.autoCompactIfNeeded(
                                client, config, List.copyOf(messageHistory), consecutiveCompactFailures
                            );
                            if (result.history().size() != messageHistory.size()) {
                                // Replace history with compacted version
                                messageHistory.clear();
                                messageHistory.addAll(result.history());
                                CompactEvent compactEvent = new CompactEvent(
                                    result.history().size(),
                                    AutoCompact.estimateTokens(result.history())
                                );
                                subscriber.onNext(compactEvent);
                                emitEvent(compactEvent);
                            }
                            consecutiveCompactFailures = result.failures();
                        }

                        // Build tool definitions
                        List<ToolDefinition> toolDefs = tools.values().stream()
                            .map(t -> new ToolDefinition(t.name(), t.description(), t.inputSchema()))
                            .toList();

                        // Wait if paused before API call
                        waitIfPaused();
                        if (!checkState()) {
                            break;
                        }

                        // Stream API response
                        List<ContentBlock> assistantContent = new ArrayList<>();
                        MessageStartEvent msgStart = new MessageStartEvent();
                        subscriber.onNext(msgStart);
                        emitEvent(msgStart);

                        try {
                            CountDownLatch streamLatch = new CountDownLatch(1);
                            List<StreamEvent> streamEvents = Collections.synchronizedList(new ArrayList<>());

                            Flow.Publisher<StreamEvent> streamPublisher = client.streamMessages(
                                List.copyOf(messageHistory),
                                systemPrompt,
                                toolDefs
                            );

                            streamPublisher.subscribe(new Flow.Subscriber<>() {
                                private Flow.Subscription subscription;

                                @Override
                                public void onSubscribe(Flow.Subscription subscription) {
                                    this.subscription = subscription;
                                    subscription.request(Long.MAX_VALUE);
                                }

                                @Override
                                public void onNext(StreamEvent event) {
                                    streamEvents.add(event);

                                    // Check stop during streaming
                                    if (!checkState()) {
                                        subscription.cancel();
                                        return;
                                    }

                                    try {
                                        waitIfPaused();
                                    } catch (InterruptedException e) {
                                        Thread.currentThread().interrupt();
                                        subscription.cancel();
                                        return;
                                    }

                                    String eventType = event.type();
                                    JsonNode data = event.data();

                                    if ("content_block_start".equals(eventType)) {
                                        ContentBlock block = parseBlock(data.get("content_block"));
                                        if (block != null) {
                                            assistantContent.add(block);
                                        }
                                    } else if ("content_block_delta".equals(eventType)) {
                                        JsonNode delta = data.get("delta");
                                        if (delta != null) {
                                            String deltaType = delta.get("type").asText();
                                            if ("text_delta".equals(deltaType)) {
                                                String text = delta.get("text").asText();
                                                MessageDeltaEvent mde = new MessageDeltaEvent(text);
                                                subscriber.onNext(mde);
                                                emitEvent(mde);
                                            } else if ("thinking_delta".equals(deltaType)) {
                                                String thinking = delta.get("thinking").asText();
                                                ThinkingDeltaEvent tde = new ThinkingDeltaEvent(thinking);
                                                subscriber.onNext(tde);
                                                emitEvent(tde);
                                            }
                                        }
                                    } else if ("message_stop".equals(eventType)) {
                                        // Message complete
                                    }
                                }

                                @Override
                                public void onError(Throwable throwable) {
                                    streamLatch.countDown();
                                }

                                @Override
                                public void onComplete() {
                                    streamLatch.countDown();
                                }
                            });

                            streamLatch.await(config.timeout().toMillis(), TimeUnit.MILLISECONDS);

                        } catch (Exception e) {
                            ErrorEvent error = new ErrorEvent("API error: " + e.getMessage());
                            subscriber.onNext(error);
                            emitEvent(error);
                            break;
                        }

                        MessageEndEvent msgEnd = new MessageEndEvent();
                        subscriber.onNext(msgEnd);
                        emitEvent(msgEnd);

                        if (!checkState()) {
                            break;
                        }

                        // Add assistant message to history
                        messageHistory.add(new Message(Role.ASSISTANT, List.copyOf(assistantContent)));

                        // Extract tool uses
                        List<ToolUseBlock> toolUses = assistantContent.stream()
                            .filter(b -> b instanceof ToolUseBlock)
                            .map(b -> (ToolUseBlock) b)
                            .toList();

                        if (toolUses.isEmpty()) {
                            // Task complete
                            String finalText = extractText(assistantContent);
                            synchronized (stateLock) {
                                state = AgentState.COMPLETED;
                            }
                            CompleteEvent complete = new CompleteEvent(finalText);
                            subscriber.onNext(complete);
                            emitEvent(complete);
                            return;
                        }

                        // Execute tools
                        List<ToolResultBlock> results = executeTools(toolUses, subscriber);

                        if (!checkState()) {
                            break;
                        }

                        // Add tool results to history
                        messageHistory.add(new Message(Role.USER,
                            results.stream().map(r -> (ContentBlock) r).toList()));
                    }

                    // Max turns reached
                    if (turnCount >= config.maxTurns()) {
                        ErrorEvent error = new ErrorEvent("Max turns reached");
                        subscriber.onNext(error);
                        emitEvent(error);
                    }

                } catch (Exception e) {
                    ErrorEvent error = new ErrorEvent(e.getMessage());
                    subscriber.onNext(error);
                    emitEvent(error);
                } finally {
                    synchronized (stateLock) {
                        if (state != AgentState.COMPLETED) {
                            state = AgentState.TERMINATED;
                        }
                    }
                    subscriber.onComplete();
                }
            });
        };
    }

    // ------------------------------------------------------------------
    // Tool execution
    // ------------------------------------------------------------------

    private List<ToolResultBlock> executeTools(
        List<ToolUseBlock> toolUses,
        Flow.Subscriber<? super Event> subscriber
    ) {
        // Group by read-only
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

        // Concurrent execution of read-only tools
        if (!readOnly.isEmpty()) {
            List<CompletableFuture<Void>> futures = readOnly.stream()
                .map(tu -> CompletableFuture.runAsync(() -> {
                    ToolResultBlock result = executeSingleTool(tu, subscriber);
                    results.add(result);
                }, executor))
                .toList();

            try {
                CompletableFuture.allOf(futures.toArray(new CompletableFuture[0]))
                    .get(config.timeout().toMillis(), TimeUnit.MILLISECONDS);
            } catch (Exception e) {
                for (ToolUseBlock tu : readOnly) {
                    boolean found = results.stream().anyMatch(r -> r.toolUseId().equals(tu.id()));
                    if (!found) {
                        results.add(new ToolResultBlock(tu.id(),
                            "Tool execution failed: " + e.getMessage(), true));
                    }
                }
            }
        }

        // Sequential execution of write tools
        for (ToolUseBlock tu : write) {
            if (!checkState()) {
                break;
            }
            results.add(executeSingleTool(tu, subscriber));
        }

        return results;
    }

    private ToolResultBlock executeSingleTool(
        ToolUseBlock tu,
        Flow.Subscriber<? super Event> subscriber
    ) {
        ToolUseStartEvent startEvent = new ToolUseStartEvent(tu.name(), tu.id(), tu.input());
        subscriber.onNext(startEvent);
        emitEvent(startEvent);

        Tool tool = tools.get(tu.name());
        if (tool == null) {
            ToolResultBlock errorResult = new ToolResultBlock(tu.id(),
                "Tool not found: " + tu.name(), true);
            ToolUseEndEvent endEvent = new ToolUseEndEvent(tu.name(), tu.id(),
                new ToolResult(errorResult.content(), true));
            subscriber.onNext(endEvent);
            emitEvent(endEvent);
            return errorResult;
        }

        Map<String, Object> input = objectMapper.convertValue(tu.input(), new TypeReference<>() {});
        ToolContext ctx = new ToolContext(config.workDir(), List.copyOf(messageHistory),
            config.allowedReadDirs(), config.allowedWriteDirs());

        ToolResult result;
        try {
            result = tool.call(input, ctx);
        } catch (Exception e) {
            result = ToolResult.error(e.getMessage());
        }

        ToolUseEndEvent endEvent = new ToolUseEndEvent(tu.name(), tu.id(), result);
        subscriber.onNext(endEvent);
        emitEvent(endEvent);

        return new ToolResultBlock(tu.id(), result.content(), result.isError());
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    private String extractText(List<ContentBlock> blocks) {
        StringBuilder sb = new StringBuilder();
        for (ContentBlock block : blocks) {
            if (block instanceof TextBlock tb) {
                sb.append(tb.text());
            }
        }
        return sb.toString();
    }

    private ContentBlock parseBlock(JsonNode data) {
        if (data == null) return null;
        String type = data.get("type").asText();
        return switch (type) {
            case "text" -> new TextBlock(data.get("text").asText());
            case "tool_use" -> new ToolUseBlock(
                data.get("name").asText(),
                data.get("id").asText(),
                data.get("input")
            );
            case "thinking" -> new ThinkingBlock(
                data.get("thinking").asText(),
                data.has("signature") ? data.get("signature").asText() : null
            );
            default -> null;
        };
    }
}
