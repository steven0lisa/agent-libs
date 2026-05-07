package com.agentlib;

/**
 * Result of a tool execution.
 */
public record ToolResult(String content, boolean isError) {

    public static ToolResult success(String content) {
        return new ToolResult(content, false);
    }

    public static ToolResult error(String content) {
        return new ToolResult(content, true);
    }
}
