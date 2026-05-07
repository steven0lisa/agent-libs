package com.agentlib.tools;

import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class WriteFileToolTest {

    @TempDir
    Path tempDir;

    @Test
    void testWriteNewFile() throws Exception {
        WriteFileTool tool = new WriteFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "new.txt", "content", "New content"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertTrue(Files.exists(tempDir.resolve("new.txt")));
        assertEquals("New content", Files.readString(tempDir.resolve("new.txt")));
    }

    @Test
    void testOverwriteExistingFile() throws Exception {
        Path file = tempDir.resolve("existing.txt");
        Files.writeString(file, "Old content");

        WriteFileTool tool = new WriteFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "existing.txt", "content", "New content"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("New content", Files.readString(file));
    }

    @Test
    void testWriteFileCreatesDirectories() throws Exception {
        WriteFileTool tool = new WriteFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "a/b/c/deep.txt", "content", "Deep content"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertTrue(Files.exists(tempDir.resolve("a/b/c/deep.txt")));
        assertEquals("Deep content", Files.readString(tempDir.resolve("a/b/c/deep.txt")));
    }

    @Test
    void testWriteFileEscapesWorkDir() {
        WriteFileTool tool = new WriteFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "../escape.txt", "content", "bad"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("escapes working directory"));
    }

    @Test
    void testWriteFileNullContent() throws Exception {
        WriteFileTool tool = new WriteFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "empty.txt"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("", Files.readString(tempDir.resolve("empty.txt")));
    }
}
