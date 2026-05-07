package tools

import (
	"context"
	"testing"

	"github.com/steven0lisa/agent-libs/go"
)

func TestSubAgentToolName(t *testing.T) {
	cfg := agentlib.DefaultConfig()
	tool := NewSubAgentTool(cfg, nil, nil)
	if tool.Name() != "subagent" {
		t.Errorf("expected name 'subagent', got %s", tool.Name())
	}
}

func TestSubAgentToolIsReadOnly(t *testing.T) {
	cfg := agentlib.DefaultConfig()
	tool := NewSubAgentTool(cfg, nil, nil)
	if !tool.IsReadOnly() {
		t.Error("expected SubAgentTool to be read-only")
	}
}

func TestSubAgentToolCallMissingTask(t *testing.T) {
	cfg := agentlib.DefaultConfig()
	tool := NewSubAgentTool(cfg, nil, nil)
	ctx := context.Background()
	input := map[string]any{}
	toolCtx := agentlib.ToolContext{WorkDir: t.TempDir()}

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.IsError {
		t.Error("expected error for missing task")
	}
}

func TestSubAgentToolInputSchema(t *testing.T) {
	cfg := agentlib.DefaultConfig()
	tool := NewSubAgentTool(cfg, nil, nil)
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
		if r == "task" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected 'task' to be required")
	}
}

func TestSubAgentToolForksConfig(t *testing.T) {
	cfg := agentlib.DefaultConfig()
	cfg.MaxTurns = 100
	cfg.SubagentMaxTurns = 50
	cfg.EnableSubagent = true

	tool := NewSubAgentTool(cfg, nil, nil)
	if tool.parentConfig.MaxTurns != 100 {
		t.Errorf("expected parent max_turns to be 100, got %d", tool.parentConfig.MaxTurns)
	}
	if tool.parentConfig.SubagentMaxTurns != 50 {
		t.Errorf("expected parent subagent_max_turns to be 50, got %d", tool.parentConfig.SubagentMaxTurns)
	}
}

func TestSubAgentToolDescription(t *testing.T) {
	cfg := agentlib.DefaultConfig()
	tool := NewSubAgentTool(cfg, nil, nil)
	if tool.Description() == "" {
		t.Error("expected non-empty description")
	}
}
