package com.agentlib;

/**
 * Emitted for each thinking delta in a streaming response.
 */
public record ThinkingDeltaEvent(String thinking) implements Event {
    @Override
    public String type() {
        return "thinking_delta";
    }
}
