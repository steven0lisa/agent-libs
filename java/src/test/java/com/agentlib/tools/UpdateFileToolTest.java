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

class UpdateFileToolTest {

    @TempDir
    Path tempDir;

    @Test
    void testUpdateFileSingleReplace() throws Exception {
        Path file = tempDir.resolve("test.txt");
        Files.writeString(file, "Hello World");

        UpdateFileTool tool = new UpdateFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "test.txt", "old_string", "World", "new_string", "Universe"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("Hello Universe", Files.readString(file));
    }

    @Test
    void testUpdateFileReplaceAll() throws Exception {
        Path file = tempDir.resolve("test.txt");
        Files.writeString(file, "foo bar foo baz foo");

        UpdateFileTool tool = new UpdateFileTool();
        ToolResult result = tool.call(
            Map.of(
                "file_path", "test.txt",
                "old_string", "foo",
                "new_string", "qux",
                "replace_all", true
            ),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("qux bar qux baz qux", Files.readString(file));
    }

    @Test
    void testUpdateFileOldStringNotFound() throws Exception {
        Path file = tempDir.resolve("test.txt");
        Files.writeString(file, "Hello World");

        UpdateFileTool tool = new UpdateFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "test.txt", "old_string", "Missing", "new_string", "X"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertEquals("old_string not found in file", result.content());
    }

    @Test
    void testUpdateFileEscapesWorkDir() {
        UpdateFileTool tool = new UpdateFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "../secret.txt", "old_string", "a", "new_string", "b"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("escapes working directory"));
    }

    @Test
    void testUpdateFileOnlyReplacesFirstOccurrence() throws Exception {
        Path file = tempDir.resolve("test.txt");
        Files.writeString(file, "foo bar foo");

        UpdateFileTool tool = new UpdateFileTool();
        ToolResult result = tool.call(
            Map.of("file_path", "test.txt", "old_string", "foo", "new_string", "qux"),
            new ToolContext(tempDir, List.of())
        );

        assertFalse(result.isError());
        assertEquals("qux bar foo", Files.readString(file));
    }
}
