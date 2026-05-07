package tools

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

// CurlTool makes HTTP requests with security policy enforcement.
type CurlTool struct {
	agentlib.BaseTool
	client    *http.Client
	Whitelist []utils.Pattern
	Blacklist []utils.Pattern
}

// NewCurlTool creates a new CurlTool with the given whitelist and blacklist.
func NewCurlTool(whitelist, blacklist []utils.Pattern) *CurlTool {
	return &CurlTool{
		client:    &http.Client{Timeout: 30 * time.Second},
		Whitelist: whitelist,
		Blacklist: blacklist,
	}
}

// Name returns the tool name.
func (c *CurlTool) Name() string { return "curl" }

// Description returns the tool description.
func (c *CurlTool) Description() string {
	return "Make an HTTP request."
}

// IsReadOnly returns true since this tool only reads data.
func (c *CurlTool) IsReadOnly() bool { return true }

// InputSchema returns the JSON schema for the tool input.
func (c *CurlTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"url": map[string]any{"type": "string"},
			"method": map[string]any{
				"type":    "string",
				"enum":    []string{"GET", "POST", "PUT", "DELETE", "PATCH"},
				"default": "GET",
			},
			"headers": map[string]any{
				"type":                 "object",
				"additionalProperties": map[string]any{"type": "string"},
			},
			"body": map[string]any{"type": "string"},
			"timeout": map[string]any{
				"type":        "integer",
				"description": "Timeout in milliseconds",
				"default":     30000,
			},
		},
		"required": []string{"url"},
	}
}

// Call executes the curl tool.
func (c *CurlTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	url, ok := input["url"].(string)
	if !ok || url == "" {
		return agentlib.Error("url is required"), nil
	}

	// Security check - enforced at execution time, not disclosed in prompt
	allowed, reason := utils.CheckSecurityPolicy(
		url,
		c.Whitelist,
		c.Blacklist,
		true, // default allow
	)
	if !allowed {
		return agentlib.Error(fmt.Sprintf("URL blocked by security policy: %s", reason)), nil
	}

	method := agentlib.GetString(input, "method", "GET")
	timeoutMs := agentlib.GetInt(input, "timeout", 30000)

	var body io.Reader
	if b, ok := input["body"].(string); ok && b != "" {
		body = strings.NewReader(b)
	}

	// Create request with timeout context
	reqCtx, cancel := context.WithTimeout(ctx, time.Duration(timeoutMs)*time.Millisecond)
	defer cancel()

	req, err := http.NewRequestWithContext(reqCtx, method, url, body)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("Request error: %v", err)), nil
	}

	// Set headers
	if headers, ok := input["headers"].(map[string]any); ok {
		for k, v := range headers {
			req.Header.Set(k, fmt.Sprintf("%v", v))
		}
	}

	// Execute request
	resp, err := c.client.Do(req)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("HTTP error: %v", err)), nil
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("Failed to read response: %v", err)), nil
	}

	return agentlib.Success(string(respBody)), nil
}
