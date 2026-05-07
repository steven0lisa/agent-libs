package com.agentlib;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class ToolResultTest {

    @Test
    void testSuccessFactory() {
        ToolResult result = ToolResult.success("Done");
        assertEquals("Done", result.content());
        assertFalse(result.isError());
    }

    @Test
    void testErrorFactory() {
        ToolResult result = ToolResult.error("Failed");
        assertEquals("Failed", result.content());
        assertTrue(result.isError());
    }

    @Test
    void testDirectConstruction() {
        ToolResult result1 = new ToolResult("content", false);
        assertFalse(result1.isError());

        ToolResult result2 = new ToolResult("error", true);
        assertTrue(result2.isError());
    }

    @Test
    void testToolContext() {
        ToolContext ctx = new ToolContext(
            java.nio.file.Paths.get("/tmp").toAbsolutePath(),
            java.util.List.of(Message.user("Hello"))
        );
        assertEquals(java.nio.file.Paths.get("/tmp").toAbsolutePath(), ctx.workDir());
        assertEquals(1, ctx.messageHistory().size());
    }
}
