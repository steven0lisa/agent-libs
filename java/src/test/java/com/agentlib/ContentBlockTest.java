package com.agentlib;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class ContentBlockTest {

    @Test
    void testTextBlock() {
        TextBlock block = new TextBlock("Hello world");
        assertEquals("text", block.type());
        assertEquals("Hello world", block.text());
    }

    @Test
    void testToolUseBlock() {
        JsonNode input = JsonNodeFactory.instance.objectNode().put("file_path", "test.txt");
        ToolUseBlock block = new ToolUseBlock("read_file", "tool_123", input);
        assertEquals("tool_use", block.type());
        assertEquals("read_file", block.name());
        assertEquals("tool_123", block.id());
        assertEquals("test.txt", block.input().get("file_path").asText());
    }

    @Test
    void testToolResultBlock() {
        ToolResultBlock block = new ToolResultBlock("tool_123", "File contents", false);
        assertEquals("tool_result", block.type());
        assertEquals("tool_123", block.toolUseId());
        assertEquals("File contents", block.content());
        assertEquals(Boolean.FALSE, block.isError());
    }

    @Test
    void testToolResultBlockWithError() {
        ToolResultBlock block = new ToolResultBlock("tool_123", "Error message", true);
        assertEquals(Boolean.TRUE, block.isError());
    }

    @Test
    void testThinkingBlock() {
        ThinkingBlock block = new ThinkingBlock("I need to think...", "sig123");
        assertEquals("thinking", block.type());
        assertEquals("I need to think...", block.thinking());
        assertEquals("sig123", block.signature());
    }

    @Test
    void testThinkingBlockWithoutSignature() {
        ThinkingBlock block = new ThinkingBlock("Thinking...", null);
        assertNull(block.signature());
    }

    @Test
    void testSealedInterface() {
        // Verify all implementations are part of the sealed interface
        ContentBlock text = new TextBlock("test");
        ContentBlock toolUse = new ToolUseBlock("tool", "id", null);
        ContentBlock toolResult = new ToolResultBlock("id", "content", false);
        ContentBlock thinking = new ThinkingBlock("think", null);

        assertNotNull(text.type());
        assertNotNull(toolUse.type());
        assertNotNull(toolResult.type());
        assertNotNull(thinking.type());
    }
}
