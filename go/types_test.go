package agentlib

import (
	"encoding/json"
	"testing"
)

func TestTextBlock(t *testing.T) {
	block := TextBlock{Text: "hello world"}
	if block.BlockType() != BlockText {
		t.Errorf("expected BlockType to be BlockText, got %s", block.BlockType())
	}
	if block.Text != "hello world" {
		t.Errorf("expected Text to be 'hello world', got %s", block.Text)
	}
}

func TestToolUseBlock(t *testing.T) {
	block := ToolUseBlock{
		Name:  "read_file",
		ID:    "tool_123",
		Input: json.RawMessage(`{"file_path": "test.txt"}`),
	}
	if block.BlockType() != BlockToolUse {
		t.Errorf("expected BlockType to be BlockToolUse, got %s", block.BlockType())
	}
	if block.Name != "read_file" {
		t.Errorf("expected Name to be 'read_file', got %s", block.Name)
	}
	if block.ID != "tool_123" {
		t.Errorf("expected ID to be 'tool_123', got %s", block.ID)
	}
}

func TestToolResultBlock(t *testing.T) {
	isErr := true
	block := ToolResultBlock{
		ToolUseID: "tool_123",
		Content:   "file contents",
		IsError:   &isErr,
	}
	if block.BlockType() != BlockToolResult {
		t.Errorf("expected BlockType to be BlockToolResult, got %s", block.BlockType())
	}
	if block.ToolUseID != "tool_123" {
		t.Errorf("expected ToolUseID to be 'tool_123', got %s", block.ToolUseID)
	}
	if block.Content != "file contents" {
		t.Errorf("expected Content to be 'file contents', got %s", block.Content)
	}
	if block.IsError == nil || !*block.IsError {
		t.Error("expected IsError to be true")
	}
}

func TestThinkingBlock(t *testing.T) {
	block := ThinkingBlock{
		Thinking:  "I need to think about this...",
		Signature: "sig123",
	}
	if block.BlockType() != BlockThinking {
		t.Errorf("expected BlockType to be BlockThinking, got %s", block.BlockType())
	}
	if block.Thinking != "I need to think about this..." {
		t.Errorf("expected Thinking to be 'I need to think about this...', got %s", block.Thinking)
	}
	if block.Signature != "sig123" {
		t.Errorf("expected Signature to be 'sig123', got %s", block.Signature)
	}
}

func TestMessageUser(t *testing.T) {
	msg := UserMessage("hello")
	if msg.Role != RoleUser {
		t.Errorf("expected Role to be RoleUser, got %s", msg.Role)
	}
	if len(msg.Content) != 1 {
		t.Fatalf("expected 1 content block, got %d", len(msg.Content))
	}
	textBlock, ok := msg.Content[0].(TextBlock)
	if !ok {
		t.Fatalf("expected TextBlock, got %T", msg.Content[0])
	}
	if textBlock.Text != "hello" {
		t.Errorf("expected text to be 'hello', got %s", textBlock.Text)
	}
}

func TestMessageAssistant(t *testing.T) {
	blocks := []ContentBlock{
		TextBlock{Text: "result"},
	}
	msg := AssistantMessage(blocks)
	if msg.Role != RoleAssistant {
		t.Errorf("expected Role to be RoleAssistant, got %s", msg.Role)
	}
	if len(msg.Content) != 1 {
		t.Fatalf("expected 1 content block, got %d", len(msg.Content))
	}
}

func TestEventString(t *testing.T) {
	evt := Event{
		Type: EventTurnStart,
		Data: map[string]any{"turn": 1},
	}
	str := evt.String()
	if str == "" {
		t.Error("expected String() to return non-empty string")
	}
	if !contains(str, "turn_start") {
		t.Errorf("expected String() to contain 'turn_start', got %s", str)
	}
}

func TestMessageMarshalUnmarshal(t *testing.T) {
	msg := Message{
		Role: RoleUser,
		Content: []ContentBlock{
			TextBlock{Text: "hello"},
			ToolUseBlock{Name: "test", ID: "id1", Input: json.RawMessage(`{"key":"val"}`)},
		},
	}

	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("failed to marshal message: %v", err)
	}

	var unmarshaled Message
	if err := json.Unmarshal(data, &unmarshaled); err != nil {
		t.Fatalf("failed to unmarshal message: %v", err)
	}

	if unmarshaled.Role != RoleUser {
		t.Errorf("expected Role to be RoleUser, got %s", unmarshaled.Role)
	}
	if len(unmarshaled.Content) != 2 {
		t.Fatalf("expected 2 content blocks, got %d", len(unmarshaled.Content))
	}

	textBlock, ok := unmarshaled.Content[0].(TextBlock)
	if !ok {
		t.Fatalf("expected TextBlock, got %T", unmarshaled.Content[0])
	}
	if textBlock.Text != "hello" {
		t.Errorf("expected text to be 'hello', got %s", textBlock.Text)
	}

	toolUseBlock, ok := unmarshaled.Content[1].(ToolUseBlock)
	if !ok {
		t.Fatalf("expected ToolUseBlock, got %T", unmarshaled.Content[1])
	}
	if toolUseBlock.Name != "test" {
		t.Errorf("expected name to be 'test', got %s", toolUseBlock.Name)
	}
}

func TestContentBlockToMap(t *testing.T) {
	tests := []struct {
		name  string
		block ContentBlock
		want  string // expected type
	}{
		{"text", TextBlock{Text: "hello"}, "text"},
		{"tool_use", ToolUseBlock{Name: "test", ID: "id1"}, "tool_use"},
		{"tool_result", ToolResultBlock{ToolUseID: "id1", Content: "result"}, "tool_result"},
		{"thinking", ThinkingBlock{Thinking: "think"}, "thinking"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			m := ContentBlockToMap(tt.block)
			if m["type"] != tt.want {
				t.Errorf("expected type %s, got %v", tt.want, m["type"])
			}
		})
	}
}

func TestParseContentBlock(t *testing.T) {
	tests := []struct {
		name string
		data map[string]any
		want ContentBlockType
	}{
		{"text", map[string]any{"type": "text", "text": "hello"}, BlockText},
		{"tool_use", map[string]any{"type": "tool_use", "name": "test", "id": "id1"}, BlockToolUse},
		{"thinking", map[string]any{"type": "thinking", "thinking": "think"}, BlockThinking},
		{"unknown", map[string]any{"type": "unknown"}, BlockText},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			block := ParseContentBlock(tt.data)
			if block.BlockType() != tt.want {
				t.Errorf("expected type %s, got %s", tt.want, block.BlockType())
			}
		})
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && containsHelper(s, substr))
}

func containsHelper(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
