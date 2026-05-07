package com.agentlib;

import org.junit.jupiter.api.Test;

import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Duration;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class AgentConfigTest {

    @Test
    void testDefaultValues() {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test-key")
            .build();

        assertEquals("https://api.anthropic.com", config.baseUrl());
        assertEquals("claude-sonnet-4-6", config.model());
        assertNotNull(config.workDir());
        assertEquals(8192, config.maxTokens());
        assertEquals(100, config.maxTurns());
        assertEquals(Duration.ofMinutes(2), config.timeout());
        assertTrue(config.stream());
        assertTrue(config.customTools().isEmpty());
        assertEquals(0, config.maxDurationMs());
        assertFalse(config.enableSubagent());
        assertEquals(50, config.subagentMaxTurns());
    }

    @Test
    void testCustomValues() {
        Path workDir = Paths.get("/tmp/work");
        AgentConfig config = AgentConfig.builder()
            .apiKey("sk-test")
            .baseUrl("https://custom.api.com")
            .model("claude-opus")
            .workDir(workDir)
            .maxTokens(4096)
            .maxTurns(50)
            .systemPrompt("Custom prompt")
            .timeout(Duration.ofSeconds(30))
            .stream(false)
            .maxDurationMs(60000)
            .enableSubagent(true)
            .subagentMaxTurns(25)
            .build();

        assertEquals("sk-test", config.apiKey());
        assertEquals("https://custom.api.com", config.baseUrl());
        assertEquals("claude-opus", config.model());
        assertEquals(workDir, config.workDir());
        assertEquals(4096, config.maxTokens());
        assertEquals(50, config.maxTurns());
        assertEquals("Custom prompt", config.systemPrompt());
        assertEquals(Duration.ofSeconds(30), config.timeout());
        assertFalse(config.stream());
        assertEquals(60000, config.maxDurationMs());
        assertTrue(config.enableSubagent());
        assertEquals(25, config.subagentMaxTurns());
    }

    @Test
    void testSecurityLists() {
        List<Pattern> bashWhitelist = List.of(new Pattern("ls *"));
        List<Pattern> bashBlacklist = List.of(new Pattern("rm *"));
        List<Pattern> curlWhitelist = List.of(new Pattern("https://api.*"));
        List<Pattern> curlBlacklist = List.of(new Pattern("http://internal.*"));

        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .bashWhitelist(bashWhitelist)
            .bashBlacklist(bashBlacklist)
            .curlWhitelist(curlWhitelist)
            .curlBlacklist(curlBlacklist)
            .build();

        assertEquals(1, config.bashWhitelist().size());
        assertEquals(1, config.bashBlacklist().size());
        assertEquals(1, config.curlWhitelist().size());
        assertEquals(1, config.curlBlacklist().size());
    }

    @Test
    void testNegativeValuesFallbackToDefaults() {
        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .maxTokens(-1)
            .maxTurns(-1)
            .build();

        assertEquals(8192, config.maxTokens());
        assertEquals(100, config.maxTurns());
    }

    @Test
    void testCustomTools() {
        Tool customTool = new Tool() {
            @Override
            public String name() { return "custom"; }
            @Override
            public String description() { return "A custom tool"; }
            @Override
            public com.fasterxml.jackson.databind.JsonNode inputSchema() {
                return com.fasterxml.jackson.databind.node.JsonNodeFactory.instance.objectNode();
            }
            @Override
            public ToolResult call(java.util.Map<String, Object> input, ToolContext context) {
                return ToolResult.success("ok");
            }
        };

        AgentConfig config = AgentConfig.builder()
            .apiKey("test")
            .customTools(List.of(customTool))
            .build();

        assertEquals(1, config.customTools().size());
        assertEquals("custom", config.customTools().get(0).name());
    }
}
