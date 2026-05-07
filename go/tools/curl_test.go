package tools

import (
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

func TestCurlToolName(t *testing.T) {
	tool := NewCurlTool(nil, nil)
	if tool.Name() != "curl" {
		t.Errorf("expected name 'curl', got %s", tool.Name())
	}
}

func TestCurlToolIsReadOnly(t *testing.T) {
	tool := NewCurlTool(nil, nil)
	if !tool.IsReadOnly() {
		t.Error("expected CurlTool to be read-only")
	}
}

func TestCurlToolCallGet(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			t.Errorf("expected GET method, got %s", r.Method)
		}
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"status": "ok"}`))
	}))
	defer server.Close()

	tool := NewCurlTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"url": server.URL + "/test",
	}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}
	if result.Content != `{"status": "ok"}` {
		t.Errorf("expected '{\"status\": \"ok\"}', got %s", result.Content)
	}
}

func TestCurlToolCallPost(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			t.Errorf("expected POST method, got %s", r.Method)
		}
		body := make([]byte, r.ContentLength)
		r.Body.Read(body)
		w.WriteHeader(http.StatusOK)
		fmt.Fprintf(w, "Received: %s", string(body))
	}))
	defer server.Close()

	tool := NewCurlTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"url":    server.URL + "/test",
		"method": "POST",
		"body":   `{"key": "value"}`,
	}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}
	if result.Content != `Received: {"key": "value"}` {
		t.Errorf("unexpected response: %s", result.Content)
	}
}

func TestCurlToolCallWithHeaders(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		auth := r.Header.Get("Authorization")
		if auth != "Bearer token123" {
			t.Errorf("expected Authorization header 'Bearer token123', got %s", auth)
		}
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("authorized"))
	}))
	defer server.Close()

	tool := NewCurlTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"url":    server.URL + "/test",
		"method": "GET",
		"headers": map[string]any{
			"Authorization": "Bearer token123",
		},
	}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}
	if result.Content != "authorized" {
		t.Errorf("expected 'authorized', got %s", result.Content)
	}
}

func TestCurlToolCallBlacklist(t *testing.T) {
	blacklist := []utils.Pattern{
		{Pattern: "evil.com", Type: "wildcard"},
	}
	tool := NewCurlTool(nil, blacklist)
	ctx := context.Background()
	input := map[string]any{
		"url": "https://evil.com/api",
	}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for blacklisted URL")
	}
}

func TestCurlToolCallWhitelist(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("ok"))
	}))
	defer server.Close()

	// Whitelist contains the test server URL pattern
	whitelist := []utils.Pattern{
		{Pattern: server.URL, Type: "wildcard"},
	}
	blacklist := []utils.Pattern{
		{Pattern: server.URL, Type: "wildcard"}, // Same as whitelist - whitelist should win
	}

	tool := NewCurlTool(whitelist, blacklist)
	ctx := context.Background()
	input := map[string]any{
		"url": server.URL + "/test",
	}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Whitelist should override blacklist, so the request should succeed
	if result.IsError {
		t.Errorf("expected whitelist to override blacklist, got error: %s", result.Content)
	}
}

func TestCurlToolCallMissingURL(t *testing.T) {
	tool := NewCurlTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing URL")
	}
}

func TestCurlToolCallInvalidURL(t *testing.T) {
	tool := NewCurlTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"url": "not-a-valid-url://",
	}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for invalid URL")
	}
}

func TestCurlToolInputSchema(t *testing.T) {
	tool := NewCurlTool(nil, nil)
	schema := tool.InputSchema()

	if schema["type"] != "object" {
		t.Errorf("expected type 'object', got %v", schema["type"])
	}

	required, ok := schema["required"].([]string)
	if !ok {
		t.Fatal("expected required to be []string")
	}
	found := false
	for _, r := range required {
		if r == "url" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected 'url' to be required")
	}
}
