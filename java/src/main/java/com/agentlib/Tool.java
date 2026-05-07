package com.agentlib;

import com.fasterxml.jackson.databind.JsonNode;
import java.util.Map;

/**
 * Interface for tools that can be registered with an Agent.
 */
public interface Tool {
    /**
     * The unique name of this tool.
     */
    String name();

    /**
     * A description of what this tool does.
     */
    String description();

    /**
     * JSON schema describing the input parameters.
     */
    JsonNode inputSchema();

    /**
     * Whether this tool is read-only (does not modify state).
     * Read-only tools can be executed concurrently.
     */
    default boolean isReadOnly() {
        return false;
    }

    /**
     * Execute the tool with the given input and context.
     */
    ToolResult call(Map<String, Object> input, ToolContext context);
}
