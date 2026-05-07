package tools

import (
	"context"
	"fmt"
	"os/exec"
	"time"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

// BashTool executes shell commands with security policy enforcement.
type BashTool struct {
	agentlib.BaseTool
	Whitelist []utils.Pattern
	Blacklist []utils.Pattern
}

// NewBashTool creates a new BashTool with the given whitelist and blacklist.
func NewBashTool(whitelist, blacklist []utils.Pattern) *BashTool {
	return &BashTool{
		Whitelist: whitelist,
		Blacklist: blacklist,
	}
}

// Name returns the tool name.
func (b *BashTool) Name() string { return "bash" }

// Description returns the tool description.
func (b *BashTool) Description() string {
	return "Execute a shell command in the working directory."
}

// InputSchema returns the JSON schema for the tool input.
func (b *BashTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"command": map[string]any{
				"type":        "string",
				"description": "The shell command to execute",
			},
			"description": map[string]any{
				"type":        "string",
				"description": "A brief description of what the command does",
			},
			"timeout": map[string]any{
				"type":        "integer",
				"description": "Timeout in milliseconds",
				"default":     120000,
			},
		},
		"required": []string{"command"},
	}
}

// Call executes the bash tool.
func (b *BashTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	command, ok := input["command"].(string)
	if !ok || command == "" {
		return agentlib.Error("command is required"), nil
	}

	// Security check - enforced at execution time, not disclosed in prompt
	allowed, reason := utils.CheckSecurityPolicy(
		command,
		b.Whitelist,
		b.Blacklist,
		true, // default allow
	)
	if !allowed {
		return agentlib.Error(fmt.Sprintf("Command blocked by security policy: %s", reason)), nil
	}

	timeoutMs := agentlib.GetInt(input, "timeout", 120000)

	// Create context with timeout
	execCtx, cancel := context.WithTimeout(ctx, time.Duration(timeoutMs)*time.Millisecond)
	defer cancel()

	// Execute command
	cmd := exec.CommandContext(execCtx, "sh", "-c", command)
	cmd.Dir = toolCtx.WorkDir

	output, err := cmd.CombinedOutput()

	// Check for timeout
	if execCtx.Err() == context.DeadlineExceeded {
		return agentlib.Error(fmt.Sprintf("Command timed out after %dms", timeoutMs)), nil
	}

	isError := err != nil
	return agentlib.ToolResult{
		Content: string(output),
		IsError: isError,
	}, nil
}
