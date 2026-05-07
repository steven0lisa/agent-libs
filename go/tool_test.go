package agentlib

import (
	"context"
	"encoding/json"
	"testing"
)

// mockTool is a test implementation of the Tool interface.
type mockTool struct {
	BaseTool
	name        string
	description string
	readOnly    bool
	result      ToolResult
}

func (m *mockTool) Name() string { return m.name }
func (m *mockTool) Description() string { return m.description }
func (m *mockTool) InputSchema() map[string]any {
	return map[string]any{
		"type":       "object",
		"properties": map[string]any{},
	}
}
func (m *mockTool) IsReadOnly() bool { return m.readOnly }
func (m *mockTool) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
	return m.result, nil
}

func TestBaseToolIsReadOnly(t *testing.T) {
	bt := BaseTool{}
	if bt.IsReadOnly() {
		t.Error("expected BaseTool.IsReadOnly() to return false")
	}
}

func TestToolResultSuccess(t *testing.T) {
	result := Success("operation completed")
	if result.IsError {
		t.Error("expected Success result to not be an error")
	}
	if result.Content != "operation completed" {
		t.Errorf("expected Content to be 'operation completed', got %s", result.Content)
	}
}

func TestToolResultError(t *testing.T) {
	result := Error("something went wrong")
	if !result.IsError {
		t.Error("expected Error result to be an error")
	}
	if result.Content != "something went wrong" {
		t.Errorf("expected Content to be 'something went wrong', got %s", result.Content)
	}
}

func TestToToolDefinition(t *testing.T) {
	tool := &mockTool{
		name:        "test_tool",
		description: "A test tool",
		readOnly:    true,
	}

	def := ToToolDefinition(tool)
	if def.Name != "test_tool" {
		t.Errorf("expected Name to be 'test_tool', got %s", def.Name)
	}
	if def.Description != "A test tool" {
		t.Errorf("expected Description to be 'A test tool', got %s", def.Description)
	}
}

func TestParseToolInput(t *testing.T) {
	raw := json.RawMessage(`{"key": "value", "num": 42}`)
	input, err := ParseToolInput(raw)
	if err != nil {
		t.Fatalf("failed to parse tool input: %v", err)
	}
	if input["key"] != "value" {
		t.Errorf("expected key to be 'value', got %v", input["key"])
	}
	if input["num"] != float64(42) {
		t.Errorf("expected num to be 42, got %v", input["num"])
	}
}

func TestParseToolInputInvalid(t *testing.T) {
	raw := json.RawMessage(`{invalid json}`)
	_, err := ParseToolInput(raw)
	if err == nil {
		t.Error("expected error for invalid JSON")
	}
}

func TestGetString(t *testing.T) {
	m := map[string]any{"key": "value", "empty": ""}
	if GetString(m, "key", "default") != "value" {
		t.Error("expected GetString to return 'value'")
	}
	if GetString(m, "missing", "default") != "default" {
		t.Error("expected GetString to return default for missing key")
	}
	if GetString(m, "empty", "default") != "" {
		t.Error("expected GetString to return empty string")
	}
}

func TestGetInt(t *testing.T) {
	m := map[string]any{
		"int":     42,
		"int64":   int64(100),
		"float64": float64(3.14),
		"float32": float32(2.5),
	}
	if GetInt(m, "int", 0) != 42 {
		t.Errorf("expected GetInt to return 42, got %d", GetInt(m, "int", 0))
	}
	if GetInt(m, "int64", 0) != 100 {
		t.Errorf("expected GetInt to return 100, got %d", GetInt(m, "int64", 0))
	}
	if GetInt(m, "float64", 0) != 3 {
		t.Errorf("expected GetInt to return 3, got %d", GetInt(m, "float64", 0))
	}
	if GetInt(m, "missing", 99) != 99 {
		t.Errorf("expected GetInt to return default 99, got %d", GetInt(m, "missing", 99))
	}
}

func TestGetBool(t *testing.T) {
	m := map[string]any{"true": true, "false": false}
	if !GetBool(m, "true", false) {
		t.Error("expected GetBool to return true")
	}
	if GetBool(m, "false", true) {
		t.Error("expected GetBool to return false")
	}
	if !GetBool(m, "missing", true) {
		t.Error("expected GetBool to return default true")
	}
}

func TestGetMap(t *testing.T) {
	inner := map[string]any{"a": "b"}
	m := map[string]any{"map": inner, "str": "hello"}
	result := GetMap(m, "map")
	if result == nil {
		t.Fatal("expected GetMap to return non-nil map")
	}
	if result["a"] != "b" {
		t.Error("expected inner map to have key 'a'")
	}
	if GetMap(m, "str") != nil {
		t.Error("expected GetMap to return nil for non-map value")
	}
	if GetMap(m, "missing") != nil {
		t.Error("expected GetMap to return nil for missing key")
	}
}

func TestMockToolCall(t *testing.T) {
	tool := &mockTool{
		name:        "mock",
		description: "mock tool",
		result:      Success("mock result"),
	}

	ctx := context.Background()
	result, err := tool.Call(ctx, map[string]any{}, ToolContext{WorkDir: "/tmp"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.Content != "mock result" {
		t.Errorf("expected 'mock result', got %s", result.Content)
	}
}
