package com.agentlib.tools;

import com.agentlib.Pattern;
import com.agentlib.SecurityUtils;
import com.agentlib.Tool;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.List;
import java.util.Map;

/**
 * Make an HTTP request with security policy enforcement.
 */
public class CurlTool implements Tool {
    private final HttpClient httpClient;
    private final List<Pattern> whitelist;
    private final List<Pattern> blacklist;

    public CurlTool() {
        this(HttpClient.newBuilder()
            .connectTimeout(Duration.ofSeconds(30))
            .followRedirects(HttpClient.Redirect.NORMAL)
            .build(),
            List.of(), List.of());
    }

    public CurlTool(HttpClient httpClient, List<Pattern> whitelist, List<Pattern> blacklist) {
        this.httpClient = httpClient;
        this.whitelist = whitelist != null ? whitelist : List.of();
        this.blacklist = blacklist != null ? blacklist : List.of();
    }

    @Override
    public String name() {
        return "curl";
    }

    @Override
    public String description() {
        return "Make an HTTP request.";
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
        ObjectNode url = JsonNodeFactory.instance.objectNode();
        url.put("type", "string");
        properties.set("url", url);

        ObjectNode method = JsonNodeFactory.instance.objectNode();
        method.put("type", "string");
        method.put("enum", "GET,POST,PUT,DELETE,PATCH");
        properties.set("method", method);

        ObjectNode headers = JsonNodeFactory.instance.objectNode();
        headers.put("type", "object");
        properties.set("headers", headers);

        ObjectNode body = JsonNodeFactory.instance.objectNode();
        body.put("type", "string");
        properties.set("body", body);

        ObjectNode timeout = JsonNodeFactory.instance.objectNode();
        timeout.put("type", "integer");
        timeout.put("description", "Timeout in milliseconds");
        timeout.put("default", 30000);
        properties.set("timeout", timeout);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("url"));
        return schema;
    }

    @Override
    @SuppressWarnings("unchecked")
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String url = (String) input.get("url");
        String method = (String) input.getOrDefault("method", "GET");
        String body = (String) input.get("body");
        int timeoutMs = 30000;
        if (input.get("timeout") instanceof Number n) {
            timeoutMs = n.intValue();
        }

        // Security check - enforced at execution time, not disclosed in prompt
        SecurityUtils.SecurityCheckResult check = SecurityUtils.checkSecurityPolicy(
            url, whitelist, blacklist, true
        );
        if (!check.allowed()) {
            return ToolResult.error("URL blocked by security policy: " + check.reason());
        }

        try {
            HttpRequest.Builder builder = HttpRequest.newBuilder()
                .uri(URI.create(url))
                .timeout(Duration.ofMillis(timeoutMs));

            HttpRequest.BodyPublisher bodyPublisher = body != null
                ? HttpRequest.BodyPublishers.ofString(body)
                : HttpRequest.BodyPublishers.noBody();

            builder.method(method, bodyPublisher);

            if (input.get("headers") instanceof Map<?, ?> headers) {
                headers.forEach((k, v) -> builder.header((String) k, String.valueOf(v)));
            }

            HttpResponse<String> response = httpClient.send(
                builder.build(), HttpResponse.BodyHandlers.ofString());

            return ToolResult.success(response.body());
        } catch (Exception e) {
            return ToolResult.error("HTTP error: " + e.getMessage());
        }
    }
}
