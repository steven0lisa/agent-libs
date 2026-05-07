package com.agentlib.tools;

import com.agentlib.Pattern;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class BashToolTest {

    @TempDir
    Path tempDir;

    @Test
    void testEchoCommand() {
        BashTool tool = new BashTool();
        ToolResult result = tool.call(
            Map.of("command", "echo Hello"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertTrue(result.content().contains("Hello"));
    }

    @Test
    void testCommandInWorkDir() throws Exception {
        // Create a file in tempDir
        java.nio.file.Files.writeString(tempDir.resolve("test.txt"), "content");

        BashTool tool = new BashTool();
        ToolResult result = tool.call(
            Map.of("command", "ls -1"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertTrue(result.content().contains("test.txt"));
    }

    @Test
    void testFailingCommand() {
        BashTool tool = new BashTool();
        ToolResult result = tool.call(
            Map.of("command", "false"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError()); // exit code != 0
    }

    @Test
    void testBlacklistBlocks() {
        List<Pattern> blacklist = List.of(new Pattern("rm *"));
        BashTool tool = new BashTool(List.of(), blacklist);

        ToolResult result = tool.call(
            Map.of("command", "rm -rf /tmp"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("blocked by security policy"));
    }

    @Test
    void testWhitelistOverridesBlacklist() {
        List<Pattern> whitelist = List.of(new Pattern("ls *"));
        List<Pattern> blacklist = List.of(new Pattern("ls -la"));
        BashTool tool = new BashTool(whitelist, blacklist);

        ToolResult result = tool.call(
            Map.of("command", "ls -la"),
            new ToolContext(tempDir, List.of())
        );

        // Whitelist takes priority, so this should be allowed
        assertFalse(result.isError());
    }

    @Test
    void testRegexBlacklist() {
        List<Pattern> blacklist = List.of(new Pattern("rm.*-rf.*", "regex"));
        BashTool tool = new BashTool(List.of(), blacklist);

        ToolResult result = tool.call(
            Map.of("command", "rm -rf /"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("blocked by security policy"));
    }

    @Test
    void testTimeout() {
        BashTool tool = new BashTool();
        ToolResult result = tool.call(
            Map.of("command", "sleep 5", "timeout", 100),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("timed out"));
    }

    @Test
    void testReadOnly() {
        BashTool tool = new BashTool();
        assertFalse(tool.isReadOnly());
    }

    @Test
    void testToolNameAndDescription() {
        BashTool tool = new BashTool();
        assertEquals("bash", tool.name());
        assertNotNull(tool.description());
        assertNotNull(tool.inputSchema());
    }
}
