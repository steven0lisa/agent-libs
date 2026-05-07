package com.agentlib;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class MessageTest {

    @Test
    void testUserMessage() {
        Message msg = Message.user("Hello");
        assertEquals(Role.USER, msg.role());
        assertEquals(1, msg.content().size());
        assertTrue(msg.content().get(0) instanceof TextBlock);
        assertEquals("Hello", ((TextBlock) msg.content().get(0)).text());
    }

    @Test
    void testAssistantMessageWithBlocks() {
        List<ContentBlock> blocks = List.of(
            new TextBlock("Let me check"),
            new ToolUseBlock("read_file", "tool_1", null)
        );
        Message msg = Message.assistant(blocks);
        assertEquals(Role.ASSISTANT, msg.role());
        assertEquals(2, msg.content().size());
    }

    @Test
    void testAssistantMessageWithText() {
        Message msg = Message.assistant("Hello there");
        assertEquals(Role.ASSISTANT, msg.role());
        assertEquals("Hello there", msg.extractText());
    }

    @Test
    void testExtractTextMultipleBlocks() {
        List<ContentBlock> blocks = List.of(
            new TextBlock("Hello "),
            new TextBlock("world")
        );
        Message msg = new Message(Role.ASSISTANT, blocks);
        assertEquals("Hello world", msg.extractText());
    }

    @Test
    void testContentIsImmutable() {
        Message msg = Message.user("Hello");
        assertThrows(UnsupportedOperationException.class, () ->
            msg.content().add(new TextBlock("extra"))
        );
    }

    @Test
    void testRoleValues() {
        assertEquals("user", Role.USER.value());
        assertEquals("assistant", Role.ASSISTANT.value());
    }
}
