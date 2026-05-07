package com.agentlib;

/**
 * Emitted when a message response finishes streaming.
 */
public record MessageEndEvent() implements Event {
    @Override
    public String type() {
        return "message_end";
    }
}
