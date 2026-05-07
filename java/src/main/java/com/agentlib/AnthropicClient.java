package com.agentlib;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import java.util.concurrent.Flow;
import java.util.concurrent.SubmissionPublisher;

/**
 * Client for Anthropic Messages API with SSE streaming support.
 */
public class AnthropicClient {
    private final AgentConfig config;
    private final HttpClient httpClient;
    private final ObjectMapper objectMapper;

    public AnthropicClient(AgentConfig config) {
        this.config = config;
        this.httpClient = HttpClient.newBuilder()
            .connectTimeout(config.timeout())
            .followRedirects(HttpClient.Redirect.NORMAL)
            .build();
        this.objectMapper = new ObjectMapper();
    }

    /**
     * Stream messages from the Anthropic API using SSE.
     */
    public Flow.Publisher<StreamEvent> streamMessages(
        List<Message> messages,
        String systemPrompt,
        List<ToolDefinition> tools
    ) {
        return subscriber -> {
            try {
                ObjectNode requestBody = buildRequestBody(messages, systemPrompt, tools, true);

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
                        if (response.statusCode() != 200) {
                            subscriber.onError(new AgentException.ApiException(
                                "HTTP " + response.statusCode() + ": " + response.body().findFirst().orElse("")
                            ));
                            return;
                        }
                        response.body().forEach(line -> {
                            if (line.startsWith("data: ")) {
                                String data = line.substring(6);
                                if ("[DONE]".equals(data)) {
                                    return;
                                }
                                try {
                                    JsonNode node = objectMapper.readTree(data);
                                    StreamEvent event = new StreamEvent(
                                        node.get("type").asText(),
                                        node
                                    );
                                    subscriber.onNext(event);
                                } catch (Exception ignored) {
                                    // Skip malformed events
                                }
                            }
                        });
                        subscriber.onComplete();
                    })
                    .exceptionally(ex -> {
                        subscriber.onError(new AgentException.ApiException(
                            ex.getMessage(), ex
                        ));
                        return null;
                    });
            } catch (Exception e) {
                subscriber.onError(new AgentException.ApiException(
                    e.getMessage(), e
                ));
            }
        };
    }

    /**
     * Send a non-streaming request to the Anthropic API.
     */
    public Message sendMessages(
        List<Message> messages,
        String systemPrompt,
        List<ToolDefinition> tools
    ) throws Exception {
        ObjectNode requestBody = buildRequestBody(messages, systemPrompt, tools, false);

        HttpRequest request = HttpRequest.newBuilder()
            .uri(URI.create(config.baseUrl() + "/v1/messages"))
            .header("Content-Type", "application/json")
            .header("x-api-key", config.apiKey())
            .header("anthropic-version", "2023-06-01")
            .POST(HttpRequest.BodyPublishers.ofString(requestBody.toString()))
            .build();

        HttpResponse<String> response = httpClient.send(
            request, HttpResponse.BodyHandlers.ofString());

        if (response.statusCode() != 200) {
            throw new AgentException.ApiException(
                "HTTP " + response.statusCode() + ": " + response.body()
            );
        }

        JsonNode body = objectMapper.readTree(response.body());
        List<ContentBlock> content = parseContentBlocks(body.get("content"));
        return new Message(Role.ASSISTANT, content);
    }

    private ObjectNode buildRequestBody(
        List<Message> messages,
        String systemPrompt,
        List<ToolDefinition> tools,
        boolean stream
    ) {
        ObjectNode requestBody = objectMapper.createObjectNode();
        requestBody.put("model", config.model());
        requestBody.put("max_tokens", config.maxTokens());
        requestBody.put("stream", stream);
        requestBody.set("messages", messagesToJson(messages));

        if (systemPrompt != null && !systemPrompt.isEmpty()) {
            requestBody.put("system", systemPrompt);
        }

        if (tools != null && !tools.isEmpty()) {
            ArrayNode toolsArray = objectMapper.createArrayNode();
            for (ToolDefinition tool : tools) {
                ObjectNode toolNode = objectMapper.createObjectNode();
                toolNode.put("name", tool.name());
                toolNode.put("description", tool.description());
                toolNode.set("input_schema", tool.inputSchema());
                toolsArray.add(toolNode);
            }
            requestBody.set("tools", toolsArray);
        }

        return requestBody;
    }

    private ArrayNode messagesToJson(List<Message> messages) {
        ArrayNode array = objectMapper.createArrayNode();
        for (Message message : messages) {
            ObjectNode msgNode = objectMapper.createObjectNode();
            msgNode.put("role", message.role().value());
            ArrayNode contentArray = objectMapper.createArrayNode();
            for (ContentBlock block : message.content()) {
                contentArray.add(blockToJson(block));
            }
            msgNode.set("content", contentArray);
            array.add(msgNode);
        }
        return array;
    }

    private ObjectNode blockToJson(ContentBlock block) {
        ObjectNode node = objectMapper.createObjectNode();
        if (block instanceof TextBlock tb) {
            node.put("type", "text");
            node.put("text", tb.text());
        } else if (block instanceof ToolUseBlock tub) {
            node.put("type", "tool_use");
            node.put("name", tub.name());
            node.put("id", tub.id());
            node.set("input", tub.input());
        } else if (block instanceof ToolResultBlock trb) {
            node.put("type", "tool_result");
            node.put("tool_use_id", trb.toolUseId());
            node.put("content", trb.content());
            if (trb.isError() != null) {
                node.put("is_error", trb.isError());
            }
        } else if (block instanceof ThinkingBlock thb) {
            node.put("type", "thinking");
            node.put("thinking", thb.thinking());
            if (thb.signature() != null) {
                node.put("signature", thb.signature());
            }
        }
        return node;
    }

    private List<ContentBlock> parseContentBlocks(JsonNode contentNode) {
        List<ContentBlock> blocks = new java.util.ArrayList<>();
        if (contentNode != null && contentNode.isArray()) {
            for (JsonNode node : contentNode) {
                String type = node.get("type").asText();
                if ("text".equals(type)) {
                    blocks.add(new TextBlock(node.get("text").asText()));
                } else if ("tool_use".equals(type)) {
                    blocks.add(new ToolUseBlock(
                        node.get("name").asText(),
                        node.get("id").asText(),
                        node.get("input")
                    ));
                } else if ("thinking".equals(type)) {
                    blocks.add(new ThinkingBlock(
                        node.get("thinking").asText(),
                        node.has("signature") ? node.get("signature").asText() : null
                    ));
                }
            }
        }
        return blocks;
    }
}
