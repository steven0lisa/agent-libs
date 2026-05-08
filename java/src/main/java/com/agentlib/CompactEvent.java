package com.agentlib;

/**
 * Emitted when the conversation history is compacted.
 */
public record CompactEvent(int messageCount, int tokenEstimate) implements Event {
    @Override
    public String type() {
        return "compact";
    }
}
