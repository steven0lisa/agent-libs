package tools

import (
	"context"
	"fmt"
	"os"
	"strings"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

// UpdateFileTool updates a file by replacing old_string with new_string.
type UpdateFileTool struct {
	agentlib.BaseTool
}

// Name returns the tool name.
func (u *UpdateFileTool) Name() string { return "update_file" }

// Description returns the tool description.
func (u *UpdateFileTool) Description() string {
	return "Update a file by replacing old_string with new_string."
}

// IsReadOnly returns false since this tool modifies files.
func (u *UpdateFileTool) IsReadOnly() bool { return false }

// InputSchema returns the JSON schema for the tool input.
func (u *UpdateFileTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"file_path": map[string]any{"type": "string"},
			"old_string": map[string]any{
				"type":        "string",
				"description": "The text to replace",
			},
			"new_string": map[string]any{
				"type":        "string",
				"description": "The replacement text",
			},
			"replace_all": map[string]any{
				"type":        "boolean",
				"default":     false,
				"description": "Replace all occurrences",
			},
		},
		"required": []string{"file_path", "old_string", "new_string"},
	}
}

// Call executes the update_file tool.
func (u *UpdateFileTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	filePath, ok := input["file_path"].(string)
	if !ok || filePath == "" {
		return agentlib.Error("file_path is required"), nil
	}

	oldStr, ok := input["old_string"].(string)
	if !ok {
		return agentlib.Error("old_string is required"), nil
	}

	newStr, ok := input["new_string"].(string)
	if !ok {
		return agentlib.Error("new_string is required"), nil
	}

	replaceAll := agentlib.GetBool(input, "replace_all", false)

	// Security: resolve path and ensure it's within working directory
	resolved, err := utils.ResolveSafePath(filePath, toolCtx.WorkDir)
	if err != nil {
		return agentlib.Error(err.Error()), nil
	}

	// Read file
	content, err := os.ReadFile(resolved)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("Failed to read file: %v", err)), nil
	}

	// Replace
	var newContent string
	if replaceAll {
		newContent = strings.ReplaceAll(string(content), oldStr, newStr)
	} else {
		newContent = strings.Replace(string(content), oldStr, newStr, 1)
	}

	if newContent == string(content) {
		return agentlib.Error("old_string not found in file"), nil
	}

	// Write file
	if err := os.WriteFile(resolved, []byte(newContent), 0644); err != nil {
		return agentlib.Error(fmt.Sprintf("Failed to write file: %v", err)), nil
	}

	return agentlib.Success(fmt.Sprintf("File updated: %s", resolved)), nil
}
