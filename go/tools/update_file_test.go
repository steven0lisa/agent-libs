package tools

import (
	"context"
	"os"
	"path/filepath"
	"testing"

	"github.com/steven0lisa/agent-libs/go"
)

func TestUpdateFileToolName(t *testing.T) {
	tool := &UpdateFileTool{}
	if tool.Name() != "update_file" {
		t.Errorf("expected name 'update_file', got %s", tool.Name())
	}
}

func TestUpdateFileToolIsReadOnly(t *testing.T) {
	tool := &UpdateFileTool{}
	if tool.IsReadOnly() {
		t.Error("expected UpdateFileTool to not be read-only")
	}
}

func TestUpdateFileToolCall(t *testing.T) {
	workDir := t.TempDir()
	testFile := filepath.Join(workDir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello world"), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	tool := &UpdateFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path":  "test.txt",
		"old_string": "world",
		"new_string": "universe",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}

	// Verify file was updated
	content, err := os.ReadFile(testFile)
	if err != nil {
		t.Fatalf("failed to read updated file: %v", err)
	}
	if string(content) != "hello universe" {
		t.Errorf("expected 'hello universe', got %s", string(content))
	}
}

func TestUpdateFileToolCallReplaceAll(t *testing.T) {
	workDir := t.TempDir()
	testFile := filepath.Join(workDir, "test.txt")
	if err := os.WriteFile(testFile, []byte("foo bar foo baz foo"), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	tool := &UpdateFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path":   "test.txt",
		"old_string":  "foo",
		"new_string":  "XXX",
		"replace_all": true,
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}

	// Verify file was updated
	content, err := os.ReadFile(testFile)
	if err != nil {
		t.Fatalf("failed to read updated file: %v", err)
	}
	if string(content) != "XXX bar XXX baz XXX" {
		t.Errorf("expected 'XXX bar XXX baz XXX', got %s", string(content))
	}
}

func TestUpdateFileToolCallSingleReplace(t *testing.T) {
	workDir := t.TempDir()
	testFile := filepath.Join(workDir, "test.txt")
	if err := os.WriteFile(testFile, []byte("foo bar foo baz"), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	tool := &UpdateFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path":  "test.txt",
		"old_string": "foo",
		"new_string": "XXX",
		// replace_all defaults to false
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}

	// Verify only first occurrence was replaced
	content, err := os.ReadFile(testFile)
	if err != nil {
		t.Fatalf("failed to read updated file: %v", err)
	}
	if string(content) != "XXX bar foo baz" {
		t.Errorf("expected 'XXX bar foo baz', got %s", string(content))
	}
}

func TestUpdateFileToolCallOldStringNotFound(t *testing.T) {
	workDir := t.TempDir()
	testFile := filepath.Join(workDir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello world"), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	tool := &UpdateFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path":  "test.txt",
		"old_string": "nonexistent",
		"new_string": "replacement",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error when old_string not found")
	}
}

func TestUpdateFileToolCallPathEscape(t *testing.T) {
	workDir := t.TempDir()

	tool := &UpdateFileTool{}
	ctx := context.Background()
	input := map[string]any{
		"file_path":  "../outside.txt",
		"old_string": "old",
		"new_string": "new",
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

func TestUpdateFileToolCallMissingParams(t *testing.T) {
	tool := &UpdateFileTool{}
	ctx := context.Background()
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	// Missing file_path
	result, err := tool.Call(ctx, map[string]any{
		"old_string": "old",
		"new_string": "new",
	}, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing file_path")
	}

	// Missing old_string
	result, err = tool.Call(ctx, map[string]any{
		"file_path":  "test.txt",
		"new_string": "new",
	}, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing old_string")
	}

	// Missing new_string
	result, err = tool.Call(ctx, map[string]any{
		"file_path":  "test.txt",
		"old_string": "old",
	}, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing new_string")
	}
}
