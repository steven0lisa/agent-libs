package com.agentlib;

import com.fasterxml.jackson.databind.JsonNode;

/**
 * Represents a single event from the Anthropic SSE stream.
 */
public record StreamEvent(
    String type,
    JsonNode data
) {}
