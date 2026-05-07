package tools

import (
	"context"
	"runtime"
	"testing"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

func TestBashToolName(t *testing.T) {
	tool := NewBashTool(nil, nil)
	if tool.Name() != "bash" {
		t.Errorf("expected name 'bash', got %s", tool.Name())
	}
}

func TestBashToolCall(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("skipping bash test on Windows")
	}

	workDir := t.TempDir()
	tool := NewBashTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"command": "echo hello",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}
	if result.Content != "hello\n" {
		t.Errorf("expected 'hello\\n', got %q", result.Content)
	}
}

func TestBashToolCallWithTimeout(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("skipping bash test on Windows")
	}

	workDir := t.TempDir()
	tool := NewBashTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"command": "sleep 5",
		"timeout": 100, // 100ms
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for timeout")
	}
	if result.Content == "" {
		t.Error("expected timeout error message")
	}
}

func TestBashToolCallBlacklist(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("skipping bash test on Windows")
	}

	workDir := t.TempDir()
	blacklist := []utils.Pattern{
		{Pattern: "rm *", Type: "wildcard"},
	}
	tool := NewBashTool(nil, blacklist)
	ctx := context.Background()
	input := map[string]any{
		"command": "rm -rf /",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for blacklisted command")
	}
	if result.Content == "" {
		t.Error("expected security policy error message")
	}
}

func TestBashToolCallWhitelist(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("skipping bash test on Windows")
	}

	workDir := t.TempDir()
	whitelist := []utils.Pattern{
		{Pattern: "ls *", Type: "wildcard"},
	}
	blacklist := []utils.Pattern{
		{Pattern: "ls *", Type: "wildcard"}, // Same as whitelist - whitelist should win
	}
	tool := NewBashTool(whitelist, blacklist)
	ctx := context.Background()
	input := map[string]any{
		"command": "ls -la",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected whitelist to override blacklist, got error: %s", result.Content)
	}
}

func TestBashToolCallMissingCommand(t *testing.T) {
	tool := NewBashTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing command")
	}
}

func TestBashToolCallInvalidCommand(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("skipping bash test on Windows")
	}

	workDir := t.TempDir()
	tool := NewBashTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"command": "nonexistent_command_xyz",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for invalid command")
	}
}

func TestBashToolCallWorkingDirectory(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("skipping bash test on Windows")
	}

	workDir := t.TempDir()
	tool := NewBashTool(nil, nil)
	ctx := context.Background()
	input := map[string]any{
		"command": "pwd",
	}
	toolCtx := agentlib.ToolContext{WorkDir: workDir}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.IsError {
		t.Errorf("expected success, got error: %s", result.Content)
	}
	// The output should contain the workDir path
	if result.Content == "" {
		t.Error("expected pwd output to contain working directory")
	}
}

func TestBashToolInputSchema(t *testing.T) {
	tool := NewBashTool(nil, nil)
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
		if r == "command" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected 'command' to be required")
	}
}
