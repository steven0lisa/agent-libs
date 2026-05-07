package com.agentlib;

/**
 * Emitted for each text delta in a streaming response.
 */
public record MessageDeltaEvent(String text) implements Event {
    @Override
    public String type() {
        return "message_delta";
    }
}
