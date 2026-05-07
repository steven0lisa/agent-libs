package tools

import (
	"context"
	"os"
	"path/filepath"
	"testing"

	"github.com/steven0lisa/agent-libs/go"
)

func TestReadFileToolName(t *testing.T) {
	tool := &ReadFileTool{}
	if tool.Name() != "read_file" {
		t.Errorf("expected name 'read_file', got %s", tool.Name())
	}
}

func TestReadFileToolIsReadOnly(t *testing.T) {
	tool := &ReadFileTool{}
	if !tool.IsReadOnly() {
		t.Error("expected ReadFileTool to be read-only")
	}
}

func TestReadFileToolCall(t *testing.T) {
	workDir := t.TempDir()
	testFile := filepath.Join(workDir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello world"), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	tool := &ReadFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path": "test.txt",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}
	if result.Content != "hello world" {
		t.Errorf("expected 'hello world', got %s", result.Content)
	}
}

func TestReadFileToolCallMissingPath(t *testing.T) {
	tool := &ReadFileTool{}
	ctx := context.Background()
	input := map[string]any{}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing file_path")
	}
}

func TestReadFileToolCallPathEscape(t *testing.T) {
	workDir := t.TempDir()
	outsideDir := t.TempDir()
	outsideFile := filepath.Join(outsideDir, "secret.txt")
	if err := os.WriteFile(outsideFile, []byte("secret"), 0644); err != nil {
		t.Fatalf("failed to create outside file: %v", err)
	}

	tool := &ReadFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path": "../" + filepath.Base(outsideDir) + "/secret.txt",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for path escape attempt")
	}
	if result.Content == "secret" {
		t.Error("should not have been able to read outside file")
	}
}

func TestReadFileToolCallWithOffsetAndLimit(t *testing.T) {
	workDir := t.TempDir()
	testFile := filepath.Join(workDir, "lines.txt")
	content := "line1\nline2\nline3\nline4\nline5\n"
	if err := os.WriteFile(testFile, []byte(content), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	tool := &ReadFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path": "lines.txt",
		"offset":    1,
		"limit":     2,
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}
	// offset=1 means skip first line (line1), limit=2 means take 2 lines
	expected := "line2\nline3"
	if result.Content != expected {
		t.Errorf("expected %q, got %q", expected, result.Content)
	}
}

func TestReadFileToolCallNonExistent(t *testing.T) {
	workDir := t.TempDir()

	tool := &ReadFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path": "nonexistent.txt",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for non-existent file")
	}
}

func TestReadFileToolInputSchema(t *testing.T) {
	tool := &ReadFileTool{}
	schema := tool.InputSchema()

	if schema["type"] != "object" {
		t.Errorf("expected type 'object', got %v", schema["type"])
	}

	properties, ok := schema["properties"].(map[string]any)
	if !ok {
		t.Fatal("expected properties to be a map")
	}
	if _, ok := properties["file_path"]; !ok {
		t.Error("expected schema to have file_path property")
	}
	if _, ok := properties["offset"]; !ok {
		t.Error("expected schema to have offset property")
	}
	if _, ok := properties["limit"]; !ok {
		t.Error("expected schema to have limit property")
	}
}
