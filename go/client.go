package agentlib

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// APIRequest represents a request to the Anthropic API.
type APIRequest struct {
	Model     string           `json:"model"`
	Messages  []Message        `json:"messages"`
	System    string           `json:"system,omitempty"`
	Tools     []ToolDefinition `json:"tools,omitempty"`
	MaxTokens int              `json:"max_tokens"`
	Stream    bool             `json:"stream,omitempty"`
}

// APIResponse represents a response from the Anthropic API.
type APIResponse struct {
	ID         string         `json:"id"`
	Type       string         `json:"type"`
	Role       Role           `json:"role"`
	Content    []map[string]any `json:"content"`
	StopReason *string        `json:"stop_reason"`
	Usage      Usage          `json:"usage"`
}

// Usage represents token usage information.
type Usage struct {
	InputTokens  int `json:"input_tokens"`
	OutputTokens int `json:"output_tokens"`
}

// APIClient is a client for the Anthropic API.
type APIClient struct {
	config     Config
	httpClient *http.Client
}

// NewAPIClient creates a new APIClient.
func NewAPIClient(cfg Config) *APIClient {
	client := cfg.HTTPClient
	if client == nil {
		client = &http.Client{
			Timeout: cfg.Timeout,
		}
	}
	return &APIClient{
		config:     cfg,
		httpClient: client,
	}
}

// StreamMessages sends a streaming request to the Anthropic API.
// Returns a channel of StreamEvent.
func (c *APIClient) StreamMessages(ctx context.Context, req APIRequest) (<-chan StreamEvent, error) {
	req.Stream = true
	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, "POST",
		c.config.BaseURL+"/v1/messages",
		bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("x-api-key", c.config.APIKey)
	httpReq.Header.Set("anthropic-version", "2023-06-01")
	httpReq.Header.Set("Accept", "text/event-stream")

	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("HTTP request failed: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		bodyBytes, _ := io.ReadAll(resp.Body)
		resp.Body.Close()
		return nil, fmt.Errorf("API returned status %d: %s", resp.StatusCode, string(bodyBytes))
	}

	events := make(chan StreamEvent, 10)
	go func() {
		defer close(events)
		defer resp.Body.Close()

		scanner := bufio.NewScanner(resp.Body)
		for scanner.Scan() {
			line := scanner.Text()
			if !strings.HasPrefix(line, "data: ") {
				continue
			}
			data := strings.TrimPrefix(line, "data: ")
			if data == "[DONE]" {
				break
			}

			var event StreamEvent
			if err := json.Unmarshal([]byte(data), &event); err != nil {
				continue
			}

			select {
			case events <- event:
			case <-ctx.Done():
				return
			}
		}
	}()

	return events, nil
}

// SendMessages sends a non-streaming request to the Anthropic API.
func (c *APIClient) SendMessages(ctx context.Context, req APIRequest) (*APIResponse, error) {
	req.Stream = false
	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, "POST",
		c.config.BaseURL+"/v1/messages",
		bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("x-api-key", c.config.APIKey)
	httpReq.Header.Set("anthropic-version", "2023-06-01")

	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("HTTP request failed: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("API returned status %d: %s", resp.StatusCode, string(respBody))
	}

	var apiResp APIResponse
	if err := json.Unmarshal(respBody, &apiResp); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &apiResp, nil
}

// MessageToMap converts a Message to a map for JSON serialization.
func MessageToMap(msg Message) map[string]any {
	content := make([]map[string]any, len(msg.Content))
	for i, block := range msg.Content {
		content[i] = ContentBlockToMap(block)
	}
	return map[string]any{
		"role":    string(msg.Role),
		"content": content,
	}
}

// ContentBlockToMap converts a ContentBlock to a map for JSON serialization.
func ContentBlockToMap(block ContentBlock) map[string]any {
	switch b := block.(type) {
	case TextBlock:
		return map[string]any{"type": "text", "text": b.Text}
	case *TextBlock:
		return map[string]any{"type": "text", "text": b.Text}
	case ToolUseBlock:
		var inputObj map[string]any
		json.Unmarshal(b.Input, &inputObj)
		return map[string]any{"type": "tool_use", "name": b.Name, "id": b.ID, "input": inputObj}
	case *ToolUseBlock:
		var inputObj map[string]any
		json.Unmarshal(b.Input, &inputObj)
		return map[string]any{"type": "tool_use", "name": b.Name, "id": b.ID, "input": inputObj}
	case ToolResultBlock:
		m := map[string]any{"type": "tool_result", "tool_use_id": b.ToolUseID, "content": b.Content}
		if b.IsError != nil {
			m["is_error"] = *b.IsError
		}
		return m
	case *ToolResultBlock:
		m := map[string]any{"type": "tool_result", "tool_use_id": b.ToolUseID, "content": b.Content}
		if b.IsError != nil {
			m["is_error"] = *b.IsError
		}
		return m
	case ThinkingBlock:
		return map[string]any{"type": "thinking", "thinking": b.Thinking, "signature": b.Signature}
	case *ThinkingBlock:
		return map[string]any{"type": "thinking", "thinking": b.Thinking, "signature": b.Signature}
	default:
		return map[string]any{"type": "text", "text": fmt.Sprintf("%v", block)}
	}
}

// ParseContentBlock parses a map into a ContentBlock.
func ParseContentBlock(data map[string]any) ContentBlock {
	blockType, _ := data["type"].(string)
	switch blockType {
	case "text":
		text, _ := data["text"].(string)
		return TextBlock{Text: text}
	case "tool_use":
		name, _ := data["name"].(string)
		id, _ := data["id"].(string)
		inputRaw, _ := json.Marshal(data["input"])
		return ToolUseBlock{Name: name, ID: id, Input: inputRaw}
	case "thinking":
		thinking, _ := data["thinking"].(string)
		signature, _ := data["signature"].(string)
		return ThinkingBlock{Thinking: thinking, Signature: signature}
	default:
		text, _ := data["text"].(string)
		return TextBlock{Text: text}
	}
}

// boolPtr returns a pointer to a bool value.
func boolPtr(b bool) *bool {
	return &b
}

// durationPtr returns a pointer to a time.Duration value.
func durationPtr(d time.Duration) *time.Duration {
	return &d
}
