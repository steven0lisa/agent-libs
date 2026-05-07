package com.agentlib;

/**
 * Emitted when the agent completes successfully.
 */
public record CompleteEvent(String finalContent) implements Event {
    @Override
    public String type() {
        return "complete";
    }
}
