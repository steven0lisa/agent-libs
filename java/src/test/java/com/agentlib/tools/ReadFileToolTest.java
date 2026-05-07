package com.agentlib.tools;

import com.agentlib.AgentException;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class ReadFileToolTest {

    @TempDir
    Path tempDir;

    @Test
    void testReadExistingFile() throws Exception {
        Path file = tempDir.resolve("test.txt");
        Files.writeString(file, "Hello, World!");

        ReadFileTool tool = new ReadFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "test.txt"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("Hello, World!", result.content());
    }

    @Test
    void testReadFileWithOffsetAndLimit() throws Exception {
        Path file = tempDir.resolve("lines.txt");
        Files.writeString(file, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5");

        ReadFileTool tool = new ReadFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "lines.txt", "offset", 2, "limit", 2),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("Line 2\nLine 3", result.content());
    }

    @Test
    void testReadNonExistentFile() {
        ReadFileTool tool = new ReadFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "nonexistent.txt"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("Failed to read file"));
    }

    @Test
    void testReadFileEscapesWorkDir() {
        ReadFileTool tool = new ReadFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "../secret.txt"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("escapes working directory"));
    }

    @Test
    void testReadFileMissingPath() {
        ReadFileTool tool = new ReadFileTool();
        ToolResult result = tool.call(
            Map.of(),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertEquals("file_path is required", result.content());
    }

    @Test
    void testReadFileInSubdirectory() throws Exception {
        Path subDir = tempDir.resolve("sub");
        Files.createDirectories(subDir);
        Files.writeString(subDir.resolve("nested.txt"), "Nested content");

        ReadFileTool tool = new ReadFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "sub/nested.txt"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("Nested content", result.content());
    }
}
