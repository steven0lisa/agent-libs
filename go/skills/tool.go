package skills

import (
	"fmt"
	"strings"
)

// SkillTool provides the "skill" tool logic. It is not itself an agentlib.Tool
// (to avoid circular imports). The agent wraps it into the proper interface.
type SkillTool struct {
	loader *Loader
}

// NewSkillTool creates a new SkillTool backed by the given loader.
func NewSkillTool(loader *Loader) *SkillTool {
	return &SkillTool{loader: loader}
}

// ToolName returns the name of the tool.
func (t *SkillTool) ToolName() string {
	return "skill"
}

// ToolDescription returns the description of the tool.
func (t *SkillTool) ToolDescription() string {
	return "Load a skill and get its instructions. Skills provide specialized capabilities for specific tasks."
}

// ToolInputSchema returns the JSON schema for the tool input.
func (t *SkillTool) ToolInputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"skill": map[string]any{
				"type":        "string",
				"description": "The name of the skill to load",
			},
			"args": map[string]any{
				"type":        "string",
				"description": "Optional arguments passed to the skill via $ARGUMENTS variable substitution",
			},
		},
		"required": []any{"skill"},
	}
}

// ToolIsReadOnly returns true since loading a skill is a read-only operation.
func (t *SkillTool) ToolIsReadOnly() bool {
	return true
}

// Execute loads a skill by name, substitutes variables, and returns the formatted result.
//
// Deprecated: Use ExecuteWithArgs instead.
func (t *SkillTool) Execute(name, args string) (string, error) {
	return t.ExecuteWithArgs(name, args)
}

// ExecuteWithArgs loads a skill by name, substitutes variables, and returns the formatted result.
func (t *SkillTool) ExecuteWithArgs(name, args string) (string, error) {
	if name == "" {
		return "", fmt.Errorf("\"skill\" is required")
	}

	skill := t.loader.FindByName(name)
	if skill == nil {
		all := t.loader.DiscoverAll()
		var names []string
		for _, s := range all {
			names = append(names, s.Metadata.Name)
		}
		available := "(none)"
		if len(names) > 0 {
			available = strings.Join(names, ", ")
		}
		return "", fmt.Errorf("Skill not found: %q. Available skills: %s", name, available)
	}

	processedContent := SubstituteVariables(skill.Content, args, skill.DirPath)
	return BuildSkillResult(skill, processedContent), nil
}

// BuildSkillResult formats a skill's metadata and processed content into a
// human-readable result string.
func BuildSkillResult(skill *SkillInfo, content string) string {
	var parts []string
	parts = append(parts, fmt.Sprintf("## Skill: %s", skill.Metadata.Name))

	if skill.Metadata.Description != "" {
		parts = append(parts, fmt.Sprintf("Description: %s", skill.Metadata.Description))
	}
	if skill.Metadata.WhenToUse != "" {
		parts = append(parts, fmt.Sprintf("When to use: %s", skill.Metadata.WhenToUse))
	}

	parts = append(parts, "")
	parts = append(parts, content)

	return strings.Join(parts, "\n")
}
