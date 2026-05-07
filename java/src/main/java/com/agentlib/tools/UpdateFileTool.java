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
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Update a file by replacing old_string with new_string.
 */
public class UpdateFileTool implements Tool {

    @Override
    public String name() {
        return "update_file";
    }

    @Override
    public String description() {
        return "Update a file by replacing old_string with new_string.";
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

        ObjectNode oldString = JsonNodeFactory.instance.objectNode();
        oldString.put("type", "string");
        oldString.put("description", "The text to replace");
        properties.set("old_string", oldString);

        ObjectNode newString = JsonNodeFactory.instance.objectNode();
        newString.put("type", "string");
        newString.put("description", "The replacement text");
        properties.set("new_string", newString);

        ObjectNode replaceAll = JsonNodeFactory.instance.objectNode();
        replaceAll.put("type", "boolean");
        replaceAll.put("default", false);
        replaceAll.put("description", "Replace all occurrences");
        properties.set("replace_all", replaceAll);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode()
            .add("file_path")
            .add("old_string")
            .add("new_string"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String filePath = (String) input.get("file_path");
        String oldStr = (String) input.get("old_string");
        String newStr = (String) input.get("new_string");
        boolean replaceAll = Boolean.TRUE.equals(input.get("replace_all"));

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
            String newContent;
            if (replaceAll) {
                newContent = content.replace(oldStr, newStr);
            } else {
                newContent = content.replaceFirst(Pattern.quote(oldStr), Matcher.quoteReplacement(newStr));
            }

            if (newContent.equals(content)) {
                return ToolResult.error("old_string not found in file");
            }

            Files.writeString(path, newContent);
            return ToolResult.success("File updated: " + path);
        } catch (IOException e) {
            return ToolResult.error("Failed to update file: " + e.getMessage());
        }
    }
}
