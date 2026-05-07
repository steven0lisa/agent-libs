package com.agentlib;

import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class EventTest {

    @Test
    void testTurnStartEvent() {
        TurnStartEvent event = new TurnStartEvent(1);
        assertEquals("turn_start", event.type());
        assertEquals(1, event.turn());
    }

    @Test
    void testMessageStartEvent() {
        MessageStartEvent event = new MessageStartEvent();
        assertEquals("message_start", event.type());
    }

    @Test
    void testMessageDeltaEvent() {
        MessageDeltaEvent event = new MessageDeltaEvent("Hello");
        assertEquals("message_delta", event.type());
        assertEquals("Hello", event.text());
    }

    @Test
    void testThinkingDeltaEvent() {
        ThinkingDeltaEvent event = new ThinkingDeltaEvent("I think...");
        assertEquals("thinking_delta", event.type());
        assertEquals("I think...", event.thinking());
    }

    @Test
    void testMessageEndEvent() {
        MessageEndEvent event = new MessageEndEvent();
        assertEquals("message_end", event.type());
    }

    @Test
    void testToolUseStartEvent() {
        var input = JsonNodeFactory.instance.objectNode().put("file_path", "test.txt");
        ToolUseStartEvent event = new ToolUseStartEvent("read_file", "tool_1", input);
        assertEquals("tool_use_start", event.type());
        assertEquals("read_file", event.name());
        assertEquals("tool_1", event.id());
    }

    @Test
    void testToolUseEndEvent() {
        ToolResult result = ToolResult.success("Done");
        ToolUseEndEvent event = new ToolUseEndEvent("read_file", "tool_1", result);
        assertEquals("tool_use_end", event.type());
        assertEquals("Done", event.result().content());
        assertFalse(event.result().isError());
    }

    @Test
    void testErrorEvent() {
        ErrorEvent event = new ErrorEvent("Something went wrong");
        assertEquals("error", event.type());
        assertEquals("Something went wrong", event.message());
    }

    @Test
    void testCompleteEvent() {
        CompleteEvent event = new CompleteEvent("Final answer");
        assertEquals("complete", event.type());
        assertEquals("Final answer", event.finalContent());
    }
}
