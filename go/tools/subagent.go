package tools

import (
	"context"
	"fmt"

	"github.com/steven0lisa/agent-libs/go"
)

// SubAgentTool creates a subagent to handle an independent task.
// The subagent forks the parent agent's context and runs with its own turn budget.
type SubAgentTool struct {
	agentlib.BaseTool
	parentConfig   agentlib.Config
	parentHistory  []agentlib.Message
	parentTools    map[string]agentlib.Tool
}

// NewSubAgentTool creates a new SubAgentTool.
func NewSubAgentTool(parentConfig agentlib.Config, parentHistory []agentlib.Message, parentTools map[string]agentlib.Tool) *SubAgentTool {
	return &SubAgentTool{
		parentConfig:  parentConfig,
		parentHistory: parentHistory,
		parentTools:   parentTools,
	}
}

// Name returns the tool name.
func (s *SubAgentTool) Name() string { return "subagent" }

// Description returns the tool description.
func (s *SubAgentTool) Description() string {
	return "Create a subagent to handle an independent task. The subagent shares your context but operates independently with its own tool budget."
}

// IsReadOnly returns true since subagents are read-only from the parent's perspective.
func (s *SubAgentTool) IsReadOnly() bool { return true }

// InputSchema returns the JSON schema for the tool input.
func (s *SubAgentTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"task": map[string]any{
				"type":        "string",
				"description": "Description of the task for the subagent",
			},
		},
		"required": []string{"task"},
	}
}

// Call executes the subagent tool.
func (s *SubAgentTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	task, ok := input["task"].(string)
	if !ok || task == "" {
		return agentlib.Error("task is required"), nil
	}

	// Fork parent config with reduced max_turns
	subConfig := agentlib.Config{
		BaseURL:          s.parentConfig.BaseURL,
		APIKey:           s.parentConfig.APIKey,
		Model:            s.parentConfig.Model,
		WorkDir:          s.parentConfig.WorkDir,
		MaxTokens:        s.parentConfig.MaxTokens,
		MaxTurns:         s.parentConfig.SubagentMaxTurns,
		MaxDuration:      s.parentConfig.MaxDuration,
		SystemPrompt:     s.parentConfig.SystemPrompt,
		Timeout:          s.parentConfig.Timeout,
		Stream:           false, // Subagents run non-streaming for simplicity
		OutputFormat:     s.parentConfig.OutputFormat,
		EnableSubagent:   false, // Prevent recursive subagents
		SubagentMaxTurns: s.parentConfig.SubagentMaxTurns,
	}

	// Create subagent
	subagent := agentlib.NewAgent(subConfig)

	// Copy parent's tool registry (except subagent itself to avoid recursion)
	for name, tool := range s.parentTools {
		if name != "subagent" {
			subagent.RegisterTool(tool)
		}
	}

	// Run subagent
	events, err := subagent.Run(ctx, task)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("Failed to start subagent: %v", err)), nil
	}

	var finalContent string
	for event := range events {
		if event.Type == agentlib.EventComplete {
			if fc, ok := event.Data["final_content"].(string); ok {
				finalContent = fc
			}
		} else if event.Type == agentlib.EventError {
			msg, _ := event.Data["message"].(string)
			return agentlib.Error(fmt.Sprintf("Subagent error: %s", msg)), nil
		}
	}

	return agentlib.Success(fmt.Sprintf("Subagent completed. Result:\n%s", finalContent)), nil
}
