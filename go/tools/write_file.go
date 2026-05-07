package tools

import (
	"context"
	"fmt"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

// WriteFileTool writes content to a file.
type WriteFileTool struct {
	agentlib.BaseTool
}

// Name returns the tool name.
func (w *WriteFileTool) Name() string { return "write_file" }

// Description returns the tool description.
func (w *WriteFileTool) Description() string {
	return "Write content to a file. Creates if not exists, overwrites if exists."
}

// IsReadOnly returns false since this tool writes files.
func (w *WriteFileTool) IsReadOnly() bool { return false }

// InputSchema returns the JSON schema for the tool input.
func (w *WriteFileTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"file_path": map[string]any{"type": "string"},
			"content":   map[string]any{"type": "string"},
		},
		"required": []string{"file_path", "content"},
	}
}

// Call executes the write_file tool.
func (w *WriteFileTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	filePath, ok := input["file_path"].(string)
	if !ok || filePath == "" {
		return agentlib.Error("file_path is required"), nil
	}

	content, ok := input["content"].(string)
	if !ok {
		return agentlib.Error("content is required"), nil
	}

	// Security: resolve path and ensure it's within working directory
	resolved, err := utils.ResolveSafePath(filePath, toolCtx.WorkDir)
	if err != nil {
		return agentlib.Error(err.Error()), nil
	}

	// Ensure directory exists
	if err := utils.EnsureDir(resolved); err != nil {
		return agentlib.Error(fmt.Sprintf("Failed to create directory: %v", err)), nil
	}

	// Write file
	if err := utils.WriteFile(resolved, []byte(content)); err != nil {
		return agentlib.Error(fmt.Sprintf("Failed to write file: %v", err)), nil
	}

	return agentlib.Success(fmt.Sprintf("File written: %s", resolved)), nil
}
