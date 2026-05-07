package com.agentlib.tools;

import com.agentlib.*;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.Flow;
import java.util.concurrent.SubmissionPublisher;

/**
 * Create a subagent to handle an independent task.
 * The subagent forks the parent agent's context (message history, tools,
 * working directory) and runs with its own turn budget.
 */
public class SubAgentTool implements Tool {
    private final AgentConfig parentConfig;
    private final List<Message> parentHistory;
    private final Map<String, Tool> parentTools;

    public SubAgentTool(AgentConfig parentConfig, List<Message> parentHistory, Map<String, Tool> parentTools) {
        this.parentConfig = parentConfig;
        this.parentHistory = parentHistory;
        this.parentTools = parentTools;
    }

    @Override
    public String name() {
        return "subagent";
    }

    @Override
    public String description() {
        return "Create a subagent to handle an independent task. " +
               "The subagent shares your context but operates independently " +
               "with its own tool budget.";
    }

    @Override
    public boolean isReadOnly() {
        return true;
    }

    @Override
    public JsonNode inputSchema() {
        ObjectNode schema = JsonNodeFactory.instance.objectNode();
        schema.put("type", "object");

        ObjectNode properties = JsonNodeFactory.instance.objectNode();
        ObjectNode task = JsonNodeFactory.instance.objectNode();
        task.put("type", "string");
        task.put("description", "Description of the task for the subagent");
        properties.set("task", task);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("task"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String task = (String) input.get("task");
        if (task == null || task.isEmpty()) {
            return ToolResult.error("task is required");
        }

        // Fork parent config with reduced max_turns
        AgentConfig subConfig = AgentConfig.builder()
            .apiKey(parentConfig.apiKey())
            .baseUrl(parentConfig.baseUrl())
            .model(parentConfig.model())
            .workDir(parentConfig.workDir())
            .maxTokens(parentConfig.maxTokens())
            .maxTurns(parentConfig.subagentMaxTurns())
            .systemPrompt(parentConfig.systemPrompt())
            .timeout(parentConfig.timeout())
            .stream(false)  // Subagents run non-streaming for simplicity
            .outputFormat(parentConfig.outputFormat())
            .build();

        // Create subagent with forked context
        Agent subagent = new Agent(subConfig);

        // Copy parent's tool registry (except subagent itself to avoid recursion)
        for (Map.Entry<String, Tool> entry : parentTools.entrySet()) {
            if (!"subagent".equals(entry.getKey())) {
                subagent.registerTool(entry.getValue());
            }
        }

        // Copy parent's message history for context
        for (Message msg : parentHistory) {
            subagent.getMessageHistory().add(msg);
        }

        // Run subagent
        StringBuilder finalContent = new StringBuilder();
        try {
            Flow.Publisher<Event> publisher = subagent.run(task);
            var latch = new java.util.concurrent.CountDownLatch(1);
            List<Event> events = new ArrayList<>();

            publisher.subscribe(new Flow.Subscriber<>() {
                private Flow.Subscription subscription;

                @Override
                public void onSubscribe(Flow.Subscription subscription) {
                    this.subscription = subscription;
                    subscription.request(Long.MAX_VALUE);
                }

                @Override
                public void onNext(Event event) {
                    events.add(event);
                    if (event instanceof CompleteEvent ce) {
                        finalContent.append(ce.finalContent());
                    }
                }

                @Override
                public void onError(Throwable throwable) {
                    latch.countDown();
                }

                @Override
                public void onComplete() {
                    latch.countDown();
                }
            });

            latch.await();

            // Check for errors
            for (Event event : events) {
                if (event instanceof ErrorEvent ee) {
                    return ToolResult.error("Subagent error: " + ee.message());
                }
            }

        } catch (Exception e) {
            return ToolResult.error("Subagent failed: " + e.getMessage());
        }

        return ToolResult.success("Subagent completed. Result:\n" + finalContent);
    }
}
