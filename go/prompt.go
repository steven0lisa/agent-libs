package agentlib

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/steven0lisa/agent-libs/go/skills"
)

const defaultSystemPrompt = `You are a helpful software engineering assistant. You have access to tools that let you interact with the file system and execute commands.

## Tool Use

You will be provided with a set of tools. When you need to perform an action, use the appropriate tool by outputting a tool_use block.

Tool use format:
- Each tool has a name and input parameters defined by a JSON schema
- To use a tool, output a content block with type "tool_use"
- The tool_use block must include: name (tool name), id (unique identifier), input (parameters as JSON object)

After using a tool, the user message will contain a tool_result block with the execution result.
- If the result indicates success, continue with your analysis or next steps
- If the result indicates an error, analyze the error and decide whether to retry, use a different approach, or ask the user for clarification

## File Operations

When working with files:
- Always read a file before editing it
- When editing files, use update_file tool with old_string/new_string for precise changes
- Use write_file to create new files
- Use read_file to inspect file contents
- All file operations are restricted to the working directory and its subdirectories

## Command Execution

When executing commands via bash:
- Prefer read-only commands for exploration (ls, grep, find, cat, etc.)
- Be careful with destructive commands (rm, dd, etc.)
- Always describe what the command does before executing
`

const subagentPrompt = `

## Subagent Capability

You can create subagents to handle independent tasks in parallel. A subagent is a separate agent instance that shares your context but operates independently.

When to use subagents:
- When you need to explore multiple approaches simultaneously
- When a task can be cleanly decomposed into independent sub-tasks
- When you want to parallelize read-only exploration

How to create a subagent:
- Use the "subagent" tool with a "task" describing what the subagent should do
- The subagent will execute with its own tool budget (max turns: %d)
- The subagent inherits your current context (files read, tool state, etc.)
- Results are returned as tool_result when the subagent completes

Important:
- Subagents run independently and do not modify your state
- File operations in subagents are still restricted to the working directory
- Prefer subagents for exploration; keep modifications in the main agent
`

// BuildSystemPrompt builds the system prompt with tool descriptions and optional skills.
func BuildSystemPrompt(tools map[string]Tool, customPrompt string, enableSubagent bool, subagentMaxTurns int, loadedSkills []skills.SkillInfo) string {
	var parts []string
	parts = append(parts, defaultSystemPrompt)

	// Tool descriptions
	var toolDescriptions []string
	for _, tool := range tools {
		schemaJSON, _ := json.Marshal(tool.InputSchema())
		toolDescriptions = append(toolDescriptions, fmt.Sprintf(
			"### %s\nDescription: %s\nInput Schema: %s",
			tool.Name(),
			tool.Description(),
			string(schemaJSON),
		))
	}

	if len(toolDescriptions) > 0 {
		parts = append(parts, "## Available Tools\n\n"+strings.Join(toolDescriptions, "\n\n"))
	}

	if enableSubagent {
		parts = append(parts, fmt.Sprintf(subagentPrompt, subagentMaxTurns))
	}

	// Skills section
	if len(loadedSkills) > 0 {
		var skillDescriptions []string
		for _, s := range loadedSkills {
			desc := s.Metadata.Name
			if s.Metadata.Description != "" {
				desc = s.Metadata.Name + " - " + s.Metadata.Description
			}
			skillDescriptions = append(skillDescriptions, desc)
		}
		parts = append(parts, "## Available Skills\n\n"+strings.Join(skillDescriptions, "\n"))
		parts = append(parts, "To use a skill, call the `skill` tool with the skill name as the `skill` parameter. Optionally pass arguments via the `args` parameter, which will be substituted for `$ARGUMENTS` in the skill content.")
	}

	if customPrompt != "" {
		parts = append(parts, "## Custom Instructions\n\n"+customPrompt)
	}

	return strings.Join(parts, "\n\n")
}
