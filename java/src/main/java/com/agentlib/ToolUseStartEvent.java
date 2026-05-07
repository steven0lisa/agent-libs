package com.agentlib;

import com.fasterxml.jackson.databind.JsonNode;

/**
 * Emitted when a tool use starts.
 */
public record ToolUseStartEvent(String name, String id, JsonNode input) implements Event {
    @Override
    public String type() {
        return "tool_use_start";
    }
}
