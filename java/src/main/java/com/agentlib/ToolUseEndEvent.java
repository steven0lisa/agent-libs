package com.agentlib;

/**
 * Emitted when a tool use completes.
 */
public record ToolUseEndEvent(String name, String id, ToolResult result) implements Event {
    @Override
    public String type() {
        return "tool_use_end";
    }
}
