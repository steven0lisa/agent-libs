package com.agentlib.tools;

import com.agentlib.SecurityUtils;
import com.agentlib.Tool;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

/**
 * Read file contents from the working directory.
 */
public class ReadFileTool implements Tool {

    @Override
    public String name() {
        return "read_file";
    }

    @Override
    public String description() {
        return "Read file contents from the working directory. Supports text, images, PDFs.";
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
        ObjectNode filePath = JsonNodeFactory.instance.objectNode();
        filePath.put("type", "string");
        filePath.put("description", "Path to the file (relative to working directory or absolute)");
        properties.set("file_path", filePath);

        ObjectNode offset = JsonNodeFactory.instance.objectNode();
        offset.put("type", "integer");
        offset.put("description", "Line number to start reading from");
        properties.set("offset", offset);

        ObjectNode limit = JsonNodeFactory.instance.objectNode();
        limit.put("type", "integer");
        limit.put("description", "Maximum number of lines to read");
        properties.set("limit", limit);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("file_path"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String filePath = (String) input.get("file_path");
        if (filePath == null || filePath.isEmpty()) {
            return ToolResult.error("file_path is required");
        }

        Path path;
        try {
            path = SecurityUtils.resolveSafePath(filePath, context.workDir());
        } catch (com.agentlib.AgentException.SecurityException e) {
            return ToolResult.error(e.getMessage());
        }

        try {
            String content = Files.readString(path);

            // Handle offset and limit
            Integer offset = null;
            Integer limit = null;
            if (input.get("offset") instanceof Number n) {
                offset = n.intValue();
            }
            if (input.get("limit") instanceof Number n) {
                limit = n.intValue();
            }

            if (offset != null || limit != null) {
                content = applyOffsetAndLimit(content, offset, limit);
            }

            return ToolResult.success(content);
        } catch (IOException e) {
            return ToolResult.error("Failed to read file: " + e.getMessage());
        }
    }

    private String applyOffsetAndLimit(String content, Integer offset, Integer limit) {
        String[] lines = content.split("\n", -1);
        int start = offset != null ? Math.max(0, offset - 1) : 0;
        int end = limit != null ? Math.min(lines.length, start + limit) : lines.length;

        if (start >= lines.length) {
            return "";
        }

        StringBuilder sb = new StringBuilder();
        for (int i = start; i < end; i++) {
            if (i > start) sb.append("\n");
            sb.append(lines[i]);
        }
        return sb.toString();
    }
}
