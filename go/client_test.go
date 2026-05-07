package agentlib

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestNewAPIClient(t *testing.T) {
	cfg := DefaultConfig()
	cfg.APIKey = "test-key"
	client := NewAPIClient(cfg)

	if client == nil {
		t.Fatal("expected NewAPIClient to return non-nil client")
	}
	if client.config.APIKey != "test-key" {
		t.Errorf("expected APIKey to be 'test-key', got %s", client.config.APIKey)
	}
}

func TestAPIClientSendMessages(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Verify request headers
		if r.Header.Get("x-api-key") != "test-key" {
			t.Errorf("expected x-api-key header to be 'test-key', got %s", r.Header.Get("x-api-key"))
		}
		if r.Header.Get("anthropic-version") != "2023-06-01" {
			t.Errorf("expected anthropic-version header to be '2023-06-01', got %s", r.Header.Get("anthropic-version"))
		}

		// Verify content type
		if r.Header.Get("Content-Type") != "application/json" {
			t.Errorf("expected Content-Type to be 'application/json', got %s", r.Header.Get("Content-Type"))
		}

		// Verify path
		if r.URL.Path != "/v1/messages" {
			t.Errorf("expected path to be '/v1/messages', got %s", r.URL.Path)
		}

		// Parse request body
		var req APIRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			t.Fatalf("failed to decode request: %v", err)
		}

		// Verify request fields
		if req.Model != "claude-sonnet-4-6" {
			t.Errorf("expected model to be 'claude-sonnet-4-6', got %s", req.Model)
		}
		if req.Stream {
			t.Error("expected Stream to be false for non-streaming request")
		}

		// Return mock response
		resp := APIResponse{
			ID:   "msg_123",
			Type: "message",
			Role: RoleAssistant,
			Content: []map[string]any{
				{"type": "text", "text": "Hello!"},
			},
			Usage: Usage{InputTokens: 10, OutputTokens: 5},
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(resp)
	}))
	defer server.Close()

	cfg := DefaultConfig()
	cfg.BaseURL = server.URL
	cfg.APIKey = "test-key"

	client := NewAPIClient(cfg)
	req := APIRequest{
		Model:     "claude-sonnet-4-6",
		Messages:  []Message{UserMessage("Hello")},
		MaxTokens: 100,
	}

	resp, err := client.SendMessages(context.Background(), req)
	if err != nil {
		t.Fatalf("SendMessages failed: %v", err)
	}

	if resp.ID != "msg_123" {
		t.Errorf("expected ID to be 'msg_123', got %s", resp.ID)
	}
	if resp.Role != RoleAssistant {
		t.Errorf("expected Role to be assistant, got %s", resp.Role)
	}
	if resp.Usage.InputTokens != 10 {
		t.Errorf("expected InputTokens to be 10, got %d", resp.Usage.InputTokens)
	}
	if resp.Usage.OutputTokens != 5 {
		t.Errorf("expected OutputTokens to be 5, got %d", resp.Usage.OutputTokens)
	}
}

func TestAPIClientSendMessagesError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusUnauthorized)
		w.Write([]byte(`{"error": {"type": "authentication_error", "message": "Invalid API key"}}`))
	}))
	defer server.Close()

	cfg := DefaultConfig()
	cfg.BaseURL = server.URL
	cfg.APIKey = "invalid-key"

	client := NewAPIClient(cfg)
	req := APIRequest{
		Model:     "claude-sonnet-4-6",
		Messages:  []Message{UserMessage("Hello")},
		MaxTokens: 100,
	}

	_, err := client.SendMessages(context.Background(), req)
	if err == nil {
		t.Error("expected error for 401 response")
	}
	if !strings.Contains(err.Error(), "401") {
		t.Errorf("expected error to contain '401', got: %v", err)
	}
}

func TestAPIClientStreamMessages(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/event-stream")
		w.WriteHeader(http.StatusOK)

		flusher, ok := w.(http.Flusher)
		if !ok {
			t.Fatal("expected ResponseWriter to support flushing")
		}

		// Send SSE events
		events := []string{
			`{"type": "message_start", "message": {"id": "msg_123", "type": "message", "role": "assistant", "content": [], "model": "claude-sonnet-4-6", "stop_reason": null, "usage": {"input_tokens": 10, "output_tokens": 0}}}`,
			`{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}`,
			`{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}`,
			`{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " world"}}`,
			`{"type": "message_stop"}`,
		}

		for _, event := range events {
			fmt.Fprintf(w, "data: %s\n\n", event)
			flusher.Flush()
		}
		fmt.Fprint(w, "data: [DONE]\n\n")
		flusher.Flush()
	}))
	defer server.Close()

	cfg := DefaultConfig()
	cfg.BaseURL = server.URL
	cfg.APIKey = "test-key"

	client := NewAPIClient(cfg)
	req := APIRequest{
		Model:     "claude-sonnet-4-6",
		Messages:  []Message{UserMessage("Hello")},
		MaxTokens: 100,
	}

	stream, err := client.StreamMessages(context.Background(), req)
	if err != nil {
		t.Fatalf("StreamMessages failed: %v", err)
	}

	var eventCount int
	for event := range stream {
		eventCount++
		if event.Type == "" && event.Message == nil && event.Delta == nil {
			// Skip empty events from unmarshal errors
			continue
		}
	}

	if eventCount == 0 {
		t.Error("expected to receive events from stream")
	}
}

func TestAPIClientStreamMessagesError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusServiceUnavailable)
		w.Write([]byte(`{"error": "Service unavailable"}`))
	}))
	defer server.Close()

	cfg := DefaultConfig()
	cfg.BaseURL = server.URL
	cfg.APIKey = "test-key"

	client := NewAPIClient(cfg)
	req := APIRequest{
		Model:     "claude-sonnet-4-6",
		Messages:  []Message{UserMessage("Hello")},
		MaxTokens: 100,
	}

	_, err := client.StreamMessages(context.Background(), req)
	if err == nil {
		t.Error("expected error for 503 response")
	}
}

func TestMessageToMap(t *testing.T) {
	msg := Message{
		Role:    RoleUser,
		Content: []ContentBlock{TextBlock{Text: "hello"}},
	}

	m := MessageToMap(msg)
	if m["role"] != "user" {
		t.Errorf("expected role to be 'user', got %v", m["role"])
	}

	content, ok := m["content"].([]map[string]any)
	if !ok {
		t.Fatalf("expected content to be []map[string]any, got %T", m["content"])
	}
	if len(content) != 1 {
		t.Fatalf("expected 1 content block, got %d", len(content))
	}
	if content[0]["type"] != "text" {
		t.Errorf("expected type to be 'text', got %v", content[0]["type"])
	}
}

func TestContentBlockToMapToolResult(t *testing.T) {
	isErr := true
	block := ToolResultBlock{
		ToolUseID: "tool_123",
		Content:   "error message",
		IsError:   &isErr,
	}

	m := ContentBlockToMap(block)
	if m["type"] != "tool_result" {
		t.Errorf("expected type to be 'tool_result', got %v", m["type"])
	}
	if m["tool_use_id"] != "tool_123" {
		t.Errorf("expected tool_use_id to be 'tool_123', got %v", m["tool_use_id"])
	}
	if m["is_error"] != true {
		t.Errorf("expected is_error to be true, got %v", m["is_error"])
	}
}

func TestContentBlockToMapThinking(t *testing.T) {
	block := ThinkingBlock{
		Thinking:  "I think...",
		Signature: "sig123",
	}

	m := ContentBlockToMap(block)
	if m["type"] != "thinking" {
		t.Errorf("expected type to be 'thinking', got %v", m["type"])
	}
	if m["thinking"] != "I think..." {
		t.Errorf("expected thinking to be 'I think...', got %v", m["thinking"])
	}
	if m["signature"] != "sig123" {
		t.Errorf("expected signature to be 'sig123', got %v", m["signature"])
	}
}

func TestParseContentBlockToolUse(t *testing.T) {
	data := map[string]any{
		"type": "tool_use",
		"name": "read_file",
		"id":   "tool_123",
		"input": map[string]any{
			"file_path": "test.txt",
		},
	}

	block := ParseContentBlock(data)
	toolUse, ok := block.(ToolUseBlock)
	if !ok {
		t.Fatalf("expected ToolUseBlock, got %T", block)
	}
	if toolUse.Name != "read_file" {
		t.Errorf("expected name to be 'read_file', got %s", toolUse.Name)
	}
}

func TestAPIClientWithCustomHTTPClient(t *testing.T) {
	customClient := &http.Client{
		Timeout: 5 * time.Second,
	}

	cfg := DefaultConfig()
	cfg.HTTPClient = customClient

	client := NewAPIClient(cfg)
	if client.httpClient != customClient {
		t.Error("expected custom HTTP client to be used")
	}
}

func TestAPIClientTimeout(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(100 * time.Millisecond)
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(APIResponse{
			ID:   "msg_123",
			Type: "message",
			Role: RoleAssistant,
		})
	}))
	defer server.Close()

	cfg := DefaultConfig()
	cfg.BaseURL = server.URL
	cfg.APIKey = "test-key"
	cfg.Timeout = 10 * time.Millisecond

	client := NewAPIClient(cfg)
	req := APIRequest{
		Model:     "claude-sonnet-4-6",
		Messages:  []Message{UserMessage("Hello")},
		MaxTokens: 100,
	}

	_, err := client.SendMessages(context.Background(), req)
	if err == nil {
		t.Error("expected timeout error")
	}
}
