package com.agentlib.tools;

import com.agentlib.Tool;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.BufferedReader;
import java.io.IOException;
import java.nio.file.FileVisitResult;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.SimpleFileVisitor;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.PatternSyntaxException;

/**
 * Search for patterns in files using regular expressions.
 */
public class GrepTool implements Tool {

    private static final int MAX_MATCHES = 100;
    private static final Set<String> SKIP_DIRS = Set.of(".git", "node_modules", ".svn", ".hg", "target", "build", "dist", ".idea", ".vscode");

    @Override
    public String name() {
        return "grep";
    }

    @Override
    public String description() {
        return "Search for patterns in files using regular expressions.";
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
        pattern.put("description", "The regular expression pattern to search for");
        properties.set("pattern", pattern);

        ObjectNode path = JsonNodeFactory.instance.objectNode();
        path.put("type", "string");
        path.put("description", "Directory or file to search in (defaults to working directory)");
        properties.set("path", path);

        ObjectNode include = JsonNodeFactory.instance.objectNode();
        include.put("type", "string");
        include.put("description", "File glob pattern to include (e.g. '*.java', '*.{ts,tsx}')");
        properties.set("include", include);

        ObjectNode ignoreCase = JsonNodeFactory.instance.objectNode();
        ignoreCase.put("type", "boolean");
        ignoreCase.put("description", "Whether to perform case-insensitive matching (default: false)");
        properties.set("ignoreCase", ignoreCase);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("pattern"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String patternStr = (String) input.get("pattern");
        if (patternStr == null || patternStr.isEmpty()) {
            return ToolResult.error("pattern is required");
        }

        String pathStr = (String) input.get("path");
        String includeGlob = (String) input.get("include");
        boolean ignoreCase = input.get("ignoreCase") instanceof Boolean b && b;

        // Compile the regex pattern
        java.util.regex.Pattern regex;
        try {
            int flags = ignoreCase ? java.util.regex.Pattern.CASE_INSENSITIVE : 0;
            regex = java.util.regex.Pattern.compile(patternStr, flags);
        } catch (PatternSyntaxException e) {
            return ToolResult.error("Invalid regex pattern: " + e.getMessage());
        }

        // Resolve the search path
        Path searchPath;
        if (pathStr != null && !pathStr.isEmpty()) {
            searchPath = context.workDir().resolve(pathStr).normalize();
        } else {
            searchPath = context.workDir();
        }

        if (!Files.exists(searchPath)) {
            return ToolResult.error("Path does not exist: " + searchPath);
        }

        // Compile include glob patterns
        List<java.util.regex.Pattern> includePatterns = parseIncludeGlob(includeGlob);

        List<String> matches = new ArrayList<>();

        try {
            if (Files.isDirectory(searchPath)) {
                Files.walkFileTree(searchPath, new SimpleFileVisitor<>() {
                    @Override
                    public FileVisitResult preVisitDirectory(Path dir, BasicFileAttributes attrs) {
                        if (SKIP_DIRS.contains(dir.getFileName().toString())) {
                            return FileVisitResult.SKIP_SUBTREE;
                        }
                        return FileVisitResult.CONTINUE;
                    }

                    @Override
                    public FileVisitResult visitFile(Path file, BasicFileAttributes attrs) throws IOException {
                        if (matches.size() >= MAX_MATCHES) {
                            return FileVisitResult.TERMINATE;
                        }

                        if (!matchesIncludePatterns(file, includePatterns)) {
                            return FileVisitResult.CONTINUE;
                        }

                        // Skip binary-like files
                        if (isBinaryFile(file)) {
                            return FileVisitResult.CONTINUE;
                        }

                        searchFile(file, regex, matches);
                        return FileVisitResult.CONTINUE;
                    }

                    @Override
                    public FileVisitResult visitFileFailed(Path file, IOException exc) {
                        // Skip files we can't read
                        return FileVisitResult.CONTINUE;
                    }
                });
            } else {
                // Search a single file
                searchFile(searchPath, regex, matches);
            }
        } catch (IOException e) {
            return ToolResult.error("Error searching files: " + e.getMessage());
        }

        if (matches.isEmpty()) {
            return ToolResult.success("No matches found.");
        }

        StringBuilder sb = new StringBuilder();
        for (String match : matches) {
            if (sb.length() > 0) sb.append("\n");
            sb.append(match);
        }

        if (matches.size() >= MAX_MATCHES) {
            sb.append("\n\n(Results truncated at ").append(MAX_MATCHES).append(" matches)");
        }

        return ToolResult.success(sb.toString());
    }

    private void searchFile(Path file, java.util.regex.Pattern regex, List<String> matches) throws IOException {
        try (BufferedReader reader = Files.newBufferedReader(file)) {
            String line;
            int lineNum = 0;
            while ((line = reader.readLine()) != null) {
                lineNum++;
                if (matches.size() >= MAX_MATCHES) {
                    break;
                }
                Matcher matcher = regex.matcher(line);
                if (matcher.find()) {
                    matches.add(file + ":" + lineNum + ": " + line);
                }
            }
        }
    }

    private List<java.util.regex.Pattern> parseIncludeGlob(String includeGlob) {
        if (includeGlob == null || includeGlob.isEmpty()) {
            return List.of();
        }

        List<java.util.regex.Pattern> patterns = new ArrayList<>();

        // Handle brace expansion like *.{java,kt}
        if (includeGlob.startsWith("*.{") && includeGlob.endsWith("}")) {
            String inner = includeGlob.substring(3, includeGlob.length() - 1);
            for (String ext : inner.split(",")) {
                String trimmed = ext.trim();
                patterns.add(java.util.regex.Pattern.compile(".*\\." + java.util.regex.Pattern.quote(trimmed.substring(trimmed.startsWith("*.") ? 2 : 0)) + "$"));
            }
        } else if (includeGlob.startsWith("*.")) {
            String ext = includeGlob.substring(2);
            patterns.add(java.util.regex.Pattern.compile(".*\\." + java.util.regex.Pattern.quote(ext) + "$"));
        } else {
            // Treat as a general glob pattern, convert to regex
            String regexStr = includeGlob
                .replace(".", "\\.")
                .replace("*", ".*")
                .replace("?", ".");
            patterns.add(java.util.regex.Pattern.compile(regexStr + "$"));
        }

        return patterns;
    }

    private boolean matchesIncludePatterns(Path file, List<java.util.regex.Pattern> includePatterns) {
        if (includePatterns.isEmpty()) {
            return true;
        }
        String fileName = file.getFileName().toString();
        for (java.util.regex.Pattern p : includePatterns) {
            if (p.matcher(fileName).matches()) {
                return true;
            }
        }
        return false;
    }

    private boolean isBinaryFile(Path file) {
        String name = file.getFileName().toString();
        // Skip common binary extensions
        return name.endsWith(".class") || name.endsWith(".jar") || name.endsWith(".war")
            || name.endsWith(".zip") || name.endsWith(".gz") || name.endsWith(".tar")
            || name.endsWith(".png") || name.endsWith(".jpg") || name.endsWith(".jpeg")
            || name.endsWith(".gif") || name.endsWith(".bmp") || name.endsWith(".ico")
            || name.endsWith(".pdf") || name.endsWith(".doc") || name.endsWith(".docx")
            || name.endsWith(".xls") || name.endsWith(".xlsx") || name.endsWith(".exe")
            || name.endsWith(".dll") || name.endsWith(".so") || name.endsWith(".dylib")
            || name.endsWith(".woff") || name.endsWith(".woff2") || name.endsWith(".ttf")
            || name.endsWith(".eot") || name.endsWith(".otf") || name.endsWith(".mp3")
            || name.endsWith(".mp4") || name.endsWith(".avi") || name.endsWith(".mov");
    }
}
