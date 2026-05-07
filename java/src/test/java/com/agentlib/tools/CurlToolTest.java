package com.agentlib.tools;

import com.agentlib.Pattern;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class CurlToolTest {

    @TempDir
    Path tempDir;

    @Test
    void testCurlLocalServer() throws Exception {
        // Start a simple local HTTP server for testing
        com.sun.net.httpserver.HttpServer server = com.sun.net.httpserver.HttpServer.create(
            new java.net.InetSocketAddress(0), 0);
        server.createContext("/test", exchange -> {
            String response = "Hello from test server";
            exchange.sendResponseHeaders(200, response.getBytes().length);
            exchange.getResponseBody().write(response.getBytes());
            exchange.close();
        });
        server.start();
        int port = server.getAddress().getPort();

        try {
            CurlTool tool = new CurlTool();
            ToolResult result = tool.call(
                Map.of("url", "http://localhost:" + port + "/test", "method", "GET"),
                new ToolContext(tempDir, List.of())
            );

            assertFalse(result.isError(), "Expected success but got: " + result.content());
            assertTrue(result.content().contains("Hello from test server"),
                "Expected test server response but got: " + result.content());
        } finally {
            server.stop(0);
        }
    }

    @Test
    void testCurlBlacklistBlocks() {
        List<Pattern> blacklist = List.of(new Pattern("https://evil.com*"));
        CurlTool tool = new CurlTool(
            java.net.http.HttpClient.newHttpClient(),
            List.of(),
            blacklist
        );

        ToolResult result = tool.call(
            Map.of("url", "https://evil.com/api"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("blocked by security policy"));
    }

    @Test
    void testCurlWhitelistOverridesBlacklist() {
        List<Pattern> whitelist = List.of(new Pattern("https://api.example.com*"));
        List<Pattern> blacklist = List.of(new Pattern("https://*.example.com*"));
        CurlTool tool = new CurlTool(
            java.net.http.HttpClient.newHttpClient(),
            whitelist,
            blacklist
        );

        // Whitelist takes priority, but the request will fail since it's not a real URL
        ToolResult result = tool.call(
            Map.of("url", "https://api.example.com/v1"),
            new ToolContext(tempDir, List.of())
        );

        // Should not be blocked by security policy (whitelist overrides)
        // But will fail due to network error
        assertFalse(result.content().contains("blocked by security policy"));
    }

    @Test
    void testCurlInvalidUrl() {
        CurlTool tool = new CurlTool();
        ToolResult result = tool.call(
            Map.of("url", "not-a-valid-url"),
            new ToolContext(tempDir, List.of())
        );

        assertTrue(result.isError());
        assertTrue(result.content().contains("HTTP error"));
    }

    @Test
    void testCurlReadOnly() {
        CurlTool tool = new CurlTool();
        assertTrue(tool.isReadOnly());
    }

    @Test
    void testCurlToolNameAndDescription() {
        CurlTool tool = new CurlTool();
        assertEquals("curl", tool.name());
        assertNotNull(tool.description());
        assertNotNull(tool.inputSchema());
    }
}
