package tools

import (
	"context"
	"fmt"
	"os"
	"strings"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

// ReadFileTool reads file contents from the working directory.
type ReadFileTool struct {
	agentlib.BaseTool
}

// Name returns the tool name.
func (r *ReadFileTool) Name() string { return "read_file" }

// Description returns the tool description.
func (r *ReadFileTool) Description() string {
	return "Read file contents from the working directory. Supports text, images, PDFs."
}

// IsReadOnly returns true since this tool only reads files.
func (r *ReadFileTool) IsReadOnly() bool { return true }

// InputSchema returns the JSON schema for the tool input.
func (r *ReadFileTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"file_path": map[string]any{
				"type":        "string",
				"description": "Path to the file (relative to working directory or absolute)",
			},
			"offset": map[string]any{
				"type":        "integer",
				"description": "Line number to start reading from",
			},
			"limit": map[string]any{
				"type":        "integer",
				"description": "Maximum number of lines to read",
			},
		},
		"required": []string{"file_path"},
	}
}

// Call executes the read_file tool.
func (r *ReadFileTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	filePath, ok := input["file_path"].(string)
	if !ok || filePath == "" {
		return agentlib.Error("file_path is required"), nil
	}

	// Security: resolve path and ensure it's within working directory
	resolved, err := utils.ResolveSafePath(filePath, toolCtx.WorkDir)
	if err != nil {
		return agentlib.Error(err.Error()), nil
	}

	content, err := os.ReadFile(resolved)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("Failed to read file: %v", err)), nil
	}

	// Handle offset and limit
	offset := agentlib.GetInt(input, "offset", 0)
	limit := agentlib.GetInt(input, "limit", 0)

	result := string(content)
	if offset > 0 || limit > 0 {
		lines := strings.Split(result, "\n")
		if offset > 0 {
			if offset >= len(lines) {
				return agentlib.Success(""), nil
			}
			lines = lines[offset:]
		}
		if limit > 0 && limit < len(lines) {
			lines = lines[:limit]
		}
		result = strings.Join(lines, "\n")
	}

	return agentlib.Success(result), nil
}
