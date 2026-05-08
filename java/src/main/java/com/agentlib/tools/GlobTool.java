package com.agentlib.tools;

import com.agentlib.Tool;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.IOException;
import java.nio.file.FileVisitResult;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.PathMatcher;
import java.nio.file.SimpleFileVisitor;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;

/**
 * Find files matching a glob pattern.
 */
public class GlobTool implements Tool {

    private static final Set<String> SKIP_DIRS = Set.of(".git", "node_modules", ".svn", ".hg", "target", "build", "dist", ".idea", ".vscode");

    @Override
    public String name() {
        return "glob";
    }

    @Override
    public String description() {
        return "Find files matching a glob pattern.";
    }

    @Override
    public boolean isReadOnly() {
        return true;
    }

    @Override
    public JsonNode inputSchema() {
        ObjectNode schema = JsonNodeFactory.instance.objectNode();
        schema.put("type", "object");

        ObjectNode properties = JsonNodeFactory.instance.objectNode();

        ObjectNode pattern = JsonNodeFactory.instance.objectNode();
        pattern.put("type", "string");
        pattern.put("description", "The glob pattern to match files against (e.g. '**/*.java', 'src/**/*.ts')");
        properties.set("pattern", pattern);

        ObjectNode path = JsonNodeFactory.instance.objectNode();
        path.put("type", "string");
        path.put("description", "Directory to search in (defaults to working directory)");
        properties.set("path", path);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("pattern"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String pattern = (String) input.get("pattern");
        if (pattern == null || pattern.isEmpty()) {
            return ToolResult.error("pattern is required");
        }

        String pathStr = (String) input.get("path");

        Path searchPath;
        if (pathStr != null && !pathStr.isEmpty()) {
            searchPath = context.workDir().resolve(pathStr).normalize();
        } else {
            searchPath = context.workDir();
        }

        if (!Files.exists(searchPath)) {
            return ToolResult.error("Path does not exist: " + searchPath);
        }

        if (!Files.isDirectory(searchPath)) {
            return ToolResult.error("Path is not a directory: " + searchPath);
        }

        // Create a PathMatcher for the glob pattern
        PathMatcher matcher;
        try {
            matcher = searchPath.getFileSystem().getPathMatcher("glob:" + pattern);
        } catch (Exception e) {
            return ToolResult.error("Invalid glob pattern: " + e.getMessage());
        }

        List<Path> matchedFiles = new ArrayList<>();

        try {
            Files.walkFileTree(searchPath, new SimpleFileVisitor<>() {
                @Override
                public FileVisitResult preVisitDirectory(Path dir, BasicFileAttributes attrs) {
                    if (SKIP_DIRS.contains(dir.getFileName().toString())) {
                        return FileVisitResult.SKIP_SUBTREE;
                    }
                    return FileVisitResult.CONTINUE;
                }

                @Override
                public FileVisitResult visitFile(Path file, BasicFileAttributes attrs) {
                    // Match against the relative path from searchPath
                    Path relative = searchPath.relativize(file);
                    if (matcher.matches(relative)) {
                        matchedFiles.add(file);
                    }
                    return FileVisitResult.CONTINUE;
                }

                @Override
                public FileVisitResult visitFileFailed(Path file, IOException exc) {
                    // Skip files we can't access
                    return FileVisitResult.CONTINUE;
                }
            });
        } catch (IOException e) {
            return ToolResult.error("Error searching files: " + e.getMessage());
        }

        if (matchedFiles.isEmpty()) {
            return ToolResult.success("No files found matching pattern: " + pattern);
        }

        // Sort by modification time (most recent first) to match the behavior of other agent libs
        matchedFiles.sort(Comparator.comparingLong((Path p) -> {
            try {
                return Files.getLastModifiedTime(p).toMillis();
            } catch (IOException e) {
                return 0L;
            }
        }).reversed());

        String result = matchedFiles.stream()
            .map(Path::toString)
            .collect(Collectors.joining("\n"));

        return ToolResult.success(result);
    }
}
