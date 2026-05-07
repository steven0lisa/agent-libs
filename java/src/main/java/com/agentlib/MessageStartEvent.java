package com.agentlib;

/**
 * Emitted when a message response starts streaming.
 */
public record MessageStartEvent() implements Event {
    @Override
    public String type() {
        return "message_start";
    }
}
