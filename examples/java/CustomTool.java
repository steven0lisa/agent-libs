package com.agentlib.examples;

import com.agentlib.*;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import java.util.Map;
import java.util.concurrent.Flow;

/**
 * Example: Register a custom tool with the agent.
 */
public class CustomTool {
    public static class CalculatorTool implements Tool {
        @Override public String name() { return "calculator"; }
        @Override public String description() { return "Perform basic arithmetic."; }
        @Override public boolean isReadOnly() { return true; }

        @Override public JsonNode inputSchema() {
            return JsonNodeFactory.instance.objectNode()
                .put("type", "object")
                .set("properties", JsonNodeFactory.instance.objectNode()
                    .set("operation", JsonNodeFactory.instance.objectNode()
                        .put("type", "string")
                        .put("enum", "add,subtract,multiply,divide"))
                    .set("a", JsonNodeFactory.instance.objectNode().put("type", "number"))
                    .set("b", JsonNodeFactory.instance.objectNode().put("type", "number")))
                .set("required", JsonNodeFactory.instance.arrayNode()
                    .add("operation").add("a").add("b"));
        }

        @Override public ToolResult call(Map<String, Object> input, ToolContext context) {
            String op = (String) input.get("operation");
            double a = ((Number) input.get("a")).doubleValue();
            double b = ((Number) input.get("b")).doubleValue();

            double result;
            switch (op) {
                case "add" -> result = a + b;
                case "subtract" -> result = a - b;
                case "multiply" -> result = a * b;
                case "divide" -> {
                    if (b == 0) return ToolResult.error("Cannot divide by zero");
                    result = a / b;
                }
                default -> { return ToolResult.error("Unknown operation: " + op); }
            }
            return ToolResult.success(String.valueOf(result));
        }
    }

    public static void main(String[] args) {
        AgentConfig config = AgentConfig.builder()
            .apiKey(System.getenv("ANTHROPIC_AUTH_TOKEN"))
            .build();

        Agent agent = new Agent(config);
        agent.registerTool(new CalculatorTool());

        agent.run("What is 123 multiplied by 456?").subscribe(new Flow.Subscriber<Event>() {
            private Flow.Subscription subscription;

            @Override public void onSubscribe(Flow.Subscription s) {
                this.subscription = s;
                s.request(Long.MAX_VALUE);
            }

            @Override public void onNext(Event event) {
                switch (event) {
                    case MessageDeltaEvent e -> System.out.print(e.getText());
                    case CompleteEvent e -> System.out.println("\n\n[Complete] " + e.getFinalContent());
                    case ErrorEvent e -> System.err.println("\n[Error] " + e.getMessage());
                    default -> {}
                }
            }

            @Override public void onError(Throwable t) { t.printStackTrace(); }
            @Override public void onComplete() {}
        });
    }
}
