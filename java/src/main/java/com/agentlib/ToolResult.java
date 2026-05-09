package com.agentlib;

import java.util.List;

/**
 * Result of a tool execution.
 */
public record ToolResult(String content, boolean isError, List<Message> newMessages) {

    public ToolResult(String content, boolean isError) {
        this(content, isError, List.of());
    }

    public static ToolResult success(String content) {
        return new ToolResult(content, false);
    }

    public static ToolResult error(String content) {
        return new ToolResult(content, true);
    }

    public static ToolResult successWithMessages(String content, List<Message> messages) {
        return new ToolResult(content, false, messages);
    }
}
