package com.agentlib;

import com.fasterxml.jackson.databind.JsonNode;
import java.util.List;

/**
 * Sealed interface representing a content block in a message.
 * Can be text, tool_use, tool_result, or thinking.
 */
public sealed interface ContentBlock {
    String type();
}

/**
 * A block of text content.
 */
record TextBlock(String text) implements ContentBlock {
    @Override
    public String type() {
        return "text";
    }
}

/**
 * A block representing a tool use request from the assistant.
 */
record ToolUseBlock(
    String name,
    String id,
    JsonNode input
) implements ContentBlock {
    @Override
    public String type() {
        return "tool_use";
    }
}

/**
 * A block representing the result of a tool execution.
 */
record ToolResultBlock(
    String toolUseId,
    String content,
    Boolean isError,
    List<Message> newMessages
) implements ContentBlock {
    ToolResultBlock(String toolUseId, String content, Boolean isError) {
        this(toolUseId, content, isError, List.of());
    }

    @Override
    public String type() {
        return "tool_result";
    }
}

/**
 * A block representing the assistant's thinking/reasoning.
 */
record ThinkingBlock(
    String thinking,
    String signature
) implements ContentBlock {
    @Override
    public String type() {
        return "thinking";
    }
}
