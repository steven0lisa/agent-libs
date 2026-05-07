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
import java.util.Map;

/**
 * Write content to a file. Creates if not exists, overwrites if exists.
 */
public class WriteFileTool implements Tool {

    @Override
    public String name() {
        return "write_file";
    }

    @Override
    public String description() {
        return "Write content to a file. Creates if not exists, overwrites if exists.";
    }

    @Override
    public boolean isReadOnly() {
        return false;
    }

    @Override
    public JsonNode inputSchema() {
        ObjectNode schema = JsonNodeFactory.instance.objectNode();
        schema.put("type", "object");

        ObjectNode properties = JsonNodeFactory.instance.objectNode();
        ObjectNode filePath = JsonNodeFactory.instance.objectNode();
        filePath.put("type", "string");
        properties.set("file_path", filePath);

        ObjectNode content = JsonNodeFactory.instance.objectNode();
        content.put("type", "string");
        properties.set("content", content);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("file_path").add("content"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String filePath = (String) input.get("file_path");
        String content = (String) input.get("content");

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
            Path parent = path.getParent();
            if (parent != null) {
                Files.createDirectories(parent);
            }
            Files.writeString(path, content != null ? content : "");
            return ToolResult.success("File written: " + path);
        } catch (IOException e) {
            return ToolResult.error("Failed to write file: " + e.getMessage());
        }
    }
}
