package com.agentlib;

/**
 * Emitted at the start of each turn.
 */
public record TurnStartEvent(int turn) implements Event {
    @Override
    public String type() {
        return "turn_start";
    }
}
