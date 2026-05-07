package com.agentlib;

import com.fasterxml.jackson.databind.JsonNode;

/**
 * Definition of a tool for the Anthropic API.
 */
public record ToolDefinition(
    String name,
    String description,
    JsonNode inputSchema
) {}
