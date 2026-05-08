package com.agentlib;

/**
 * Sealed interface representing an event emitted during agent execution.
 */
public sealed interface Event permits
    TurnStartEvent, MessageStartEvent, MessageDeltaEvent, ThinkingDeltaEvent,
    MessageEndEvent, ToolUseStartEvent, ToolUseEndEvent, ErrorEvent, CompleteEvent,
    CompactEvent {
    String type();
}
