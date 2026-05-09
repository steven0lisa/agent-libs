package agentlib

import (
	"context"
	"encoding/json"
)

// Tool is the interface for tools that can be registered with an Agent.
type Tool interface {
	Name() string
	Description() string
	InputSchema() map[string]any
	IsReadOnly() bool
	Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error)
}

// ToolContext provides context for tool execution.
type ToolContext struct {
	WorkDir        string
	MessageHistory []Message
}

// ToolResult is the result of a tool execution.
type ToolResult struct {
	Content     string    `json:"content"`
	IsError     bool      `json:"is_error"`
	NewMessages []Message `json:"new_messages,omitempty"`
}

// Success creates a successful tool result.
func Success(content string) ToolResult {
	return ToolResult{Content: content, IsError: false}
}

// Error creates an error tool result.
func Error(content string) ToolResult {
	return ToolResult{Content: content, IsError: true}
}

// SuccessWithMessages creates a successful tool result with messages to inject into the conversation.
func SuccessWithMessages(content string, messages []Message) ToolResult {
	return ToolResult{Content: content, IsError: false, NewMessages: messages}
}

// BaseTool provides default implementations for the Tool interface.
// Embed this in your custom tool structs to reduce boilerplate.
type BaseTool struct{}

// IsReadOnly returns false by default.
func (b BaseTool) IsReadOnly() bool { return false }

// ToolDefinition represents a tool definition for the API.
type ToolDefinition struct {
	Name        string         `json:"name"`
	Description string         `json:"description"`
	InputSchema map[string]any `json:"input_schema"`
}

// ToToolDefinition converts a Tool to a ToolDefinition.
func ToToolDefinition(tool Tool) ToolDefinition {
	return ToolDefinition{
		Name:        tool.Name(),
		Description: tool.Description(),
		InputSchema: tool.InputSchema(),
	}
}

// ParseToolInput parses the raw JSON input into a map.
func ParseToolInput(raw json.RawMessage) (map[string]any, error) {
	var input map[string]any
	if err := json.Unmarshal(raw, &input); err != nil {
		return nil, err
	}
	return input, nil
}

// GetString extracts a string value from a map, returning the default if not found or wrong type.
func GetString(m map[string]any, key string, defaultValue string) string {
	if v, ok := m[key].(string); ok {
		return v
	}
	return defaultValue
}

// GetInt extracts an int value from a map, returning the default if not found or wrong type.
func GetInt(m map[string]any, key string, defaultValue int) int {
	switch v := m[key].(type) {
	case int:
		return v
	case int64:
		return int(v)
	case float64:
		return int(v)
	case float32:
		return int(v)
	}
	return defaultValue
}

// GetBool extracts a bool value from a map, returning the default if not found or wrong type.
func GetBool(m map[string]any, key string, defaultValue bool) bool {
	if v, ok := m[key].(bool); ok {
		return v
	}
	return defaultValue
}

// GetMap extracts a map value from a map, returning nil if not found or wrong type.
func GetMap(m map[string]any, key string) map[string]any {
	if v, ok := m[key].(map[string]any); ok {
		return v
	}
	return nil
}
