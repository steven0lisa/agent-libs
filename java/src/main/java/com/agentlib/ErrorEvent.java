package com.agentlib;

/**
 * Emitted when an error occurs.
 */
public record ErrorEvent(String message) implements Event {
    @Override
    public String type() {
        return "error";
    }
}
