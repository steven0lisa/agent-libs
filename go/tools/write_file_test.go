package tools

import (
	"context"
	"os"
	"path/filepath"
	"testing"

	"github.com/steven0lisa/agent-libs/go"
)

func TestWriteFileToolName(t *testing.T) {
	tool := &WriteFileTool{}
	if tool.Name() != "write_file" {
		t.Errorf("expected name 'write_file', got %s", tool.Name())
	}
}

func TestWriteFileToolIsReadOnly(t *testing.T) {
	tool := &WriteFileTool{}
	if tool.IsReadOnly() {
		t.Error("expected WriteFileTool to not be read-only")
	}
}

func TestWriteFileToolCall(t *testing.T) {
	workDir := t.TempDir()

	tool := &WriteFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path": "test.txt",
		"content":   "hello world",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}

	// Verify file was written
	content, err := os.ReadFile(filepath.Join(workDir, "test.txt"))
	if err != nil {
		t.Fatalf("failed to read written file: %v", err)
	}
	if string(content) != "hello world" {
		t.Errorf("expected 'hello world', got %s", string(content))
	}
}

func TestWriteFileToolCallNestedPath(t *testing.T) {
	workDir := t.TempDir()

	tool := &WriteFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path": "subdir/nested/file.txt",
		"content":   "nested content",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}

	// Verify file was written
	content, err := os.ReadFile(filepath.Join(workDir, "subdir", "nested", "file.txt"))
	if err != nil {
		t.Fatalf("failed to read written file: %v", err)
	}
	if string(content) != "nested content" {
		t.Errorf("expected 'nested content', got %s", string(content))
	}
}

func TestWriteFileToolCallPathEscape(t *testing.T) {
	workDir := t.TempDir()
	outsideDir := t.TempDir()

	tool := &WriteFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path": "../" + filepath.Base(outsideDir) + "/evil.txt",
		"content":   "evil content",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for path escape attempt")
	}
}

func TestWriteFileToolCallMissingParams(t *testing.T) {
	tool := &WriteFileTool{}
	ctx := context.Background()

	// Missing file_path
	result, err := tool.Call(ctx, map[string]any{"content": "hello"}, agentlib.ToolContext{WorkDir: t.TempDir()})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing file_path")
	}

	// Missing content
	result, err = tool.Call(ctx, map[string]any{"file_path": "test.txt"}, agentlib.ToolContext{WorkDir: t.TempDir()})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing content")
	}
}

func TestWriteFileToolInputSchema(t *testing.T) {
	tool := &WriteFileTool{}
	schema := tool.InputSchema()

	if schema["type"] != "object" {
		t.Errorf("expected type 'object', got %v", schema["type"])
	}

	required, ok := schema["required"].([]string)
	if !ok {
		t.Fatal("expected required to be []string")
	}
	if len(required) != 2 {
		t.Errorf("expected 2 required fields, got %d", len(required))
	}
}
