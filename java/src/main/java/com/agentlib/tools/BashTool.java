package com.agentlib.tools;

import com.agentlib.Pattern;
import com.agentlib.SecurityUtils;
import com.agentlib.Tool;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.concurrent.TimeUnit;

/**
 * Execute a shell command with security policy enforcement.
 */
public class BashTool implements Tool {
    private final List<Pattern> whitelist;
    private final List<Pattern> blacklist;

    public BashTool() {
        this(List.of(), List.of());
    }

    public BashTool(List<Pattern> whitelist, List<Pattern> blacklist) {
        this.whitelist = whitelist != null ? whitelist : List.of();
        this.blacklist = blacklist != null ? blacklist : List.of();
    }

    @Override
    public String name() {
        return "bash";
    }

    @Override
    public String description() {
        return "Execute a shell command in the working directory.";
    }

    @Override
    public JsonNode inputSchema() {
        ObjectNode schema = JsonNodeFactory.instance.objectNode();
        schema.put("type", "object");

        ObjectNode properties = JsonNodeFactory.instance.objectNode();
        ObjectNode command = JsonNodeFactory.instance.objectNode();
        command.put("type", "string");
        command.put("description", "The shell command to execute");
        properties.set("command", command);

        ObjectNode description = JsonNodeFactory.instance.objectNode();
        description.put("type", "string");
        description.put("description", "A brief description of what the command does");
        properties.set("description", description);

        ObjectNode timeout = JsonNodeFactory.instance.objectNode();
        timeout.put("type", "integer");
        timeout.put("description", "Timeout in milliseconds");
        timeout.put("default", 120000);
        properties.set("timeout", timeout);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("command"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String command = (String) input.get("command");
        int timeoutMs = 120000;
        if (input.get("timeout") instanceof Number n) {
            timeoutMs = n.intValue();
        }

        // Security check - enforced at execution time, not disclosed in prompt
        SecurityUtils.SecurityCheckResult check = SecurityUtils.checkSecurityPolicy(
            command, whitelist, blacklist, true
        );
        if (!check.allowed()) {
            return ToolResult.error("Command blocked by security policy: " + check.reason());
        }

        try {
            ProcessBuilder pb = new ProcessBuilder("sh", "-c", command)
                .directory(context.workDir().toFile())
                .redirectErrorStream(true);

            Process process = pb.start();
            boolean finished = process.waitFor(timeoutMs, TimeUnit.MILLISECONDS);

            if (!finished) {
                process.destroyForcibly();
                return ToolResult.error("Command timed out after " + timeoutMs + "ms");
            }

            String output = new String(process.getInputStream().readAllBytes(), StandardCharsets.UTF_8);
            return new ToolResult(output, process.exitValue() != 0);
        } catch (Exception e) {
            return ToolResult.error("Failed to execute: " + e.getMessage());
        }
    }
}
