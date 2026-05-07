package com.agentlib;

import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class PromptBuilderTest {

    @Test
    void testBuildPromptContainsDefaultInstructions() {
        Tool dummyTool = new Tool() {
            @Override
            public String name() { return "dummy"; }
            @Override
            public String description() { return "A dummy tool"; }
            @Override
            public com.fasterxml.jackson.databind.JsonNode inputSchema() {
                return JsonNodeFactory.instance.objectNode();
            }
            @Override
            public ToolResult call(java.util.Map<String, Object> input, ToolContext context) {
                return ToolResult.success("ok");
            }
        };

        String prompt = PromptBuilder.build(List.of(dummyTool), null, false, 50);

        assertTrue(prompt.contains("Tool Use"));
        assertTrue(prompt.contains("File Operations"));
        assertTrue(prompt.contains("Command Execution"));
        assertTrue(prompt.contains("Available Tools"));
        assertTrue(prompt.contains("dummy"));
        assertTrue(prompt.contains("A dummy tool"));
    }

    @Test
    void testBuildPromptWithCustomPrompt() {
        String prompt = PromptBuilder.build(List.of(), "Be extra helpful", false, 50);

        assertTrue(prompt.contains("Custom Instructions"));
        assertTrue(prompt.contains("Be extra helpful"));
    }

    @Test
    void testBuildPromptWithSubagent() {
        String prompt = PromptBuilder.build(List.of(), null, true, 30);

        assertTrue(prompt.contains("Subagent Capability"));
        assertTrue(prompt.contains("30"));
    }

    @Test
    void testBuildPromptWithoutSubagent() {
        String prompt = PromptBuilder.build(List.of(), null, false, 50);

        assertFalse(prompt.contains("Subagent Capability"));
    }

    @Test
    void testBuildPromptWithMultipleTools() {
        Tool tool1 = new Tool() {
            @Override
            public String name() { return "tool1"; }
            @Override
            public String description() { return "First tool"; }
            @Override
            public com.fasterxml.jackson.databind.JsonNode inputSchema() {
                return JsonNodeFactory.instance.objectNode();
            }
            @Override
            public ToolResult call(java.util.Map<String, Object> input, ToolContext context) {
                return ToolResult.success("ok");
            }
        };

        Tool tool2 = new Tool() {
            @Override
            public String name() { return "tool2"; }
            @Override
            public String description() { return "Second tool"; }
            @Override
            public com.fasterxml.jackson.databind.JsonNode inputSchema() {
                return JsonNodeFactory.instance.objectNode();
            }
            @Override
            public ToolResult call(java.util.Map<String, Object> input, ToolContext context) {
                return ToolResult.success("ok");
            }
        };

        String prompt = PromptBuilder.build(List.of(tool1, tool2), null, false, 50);

        assertTrue(prompt.contains("tool1"));
        assertTrue(prompt.contains("tool2"));
        assertTrue(prompt.contains("First tool"));
        assertTrue(prompt.contains("Second tool"));
    }
}
