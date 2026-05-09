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

// ToolDescription returns the description of the tool, dynamically listing
// user-invocable skills.
func (t *SkillTool) ToolDescription() string {
	desc := "Load a skill and get its instructions. Skills provide specialized capabilities for specific tasks."
	skills := t.loader.DiscoverAll()
	var invocable []SkillInfo
	for _, s := range skills {
		if s.Metadata.UserInvocable {
			invocable = append(invocable, s)
		}
	}
	if len(invocable) > 0 {
		desc += "\n\nAvailable skills:\n"
		for _, skill := range invocable {
			if skill.Metadata.Description != "" {
				desc += fmt.Sprintf("- %s: %s\n", skill.Metadata.Name, skill.Metadata.Description)
			} else {
				desc += fmt.Sprintf("- %s\n", skill.Metadata.Name)
			}
		}
	}
	return desc
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
// Deprecated: Use ExecuteWithInjection instead.
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
	return BuildSkillInjectionContent(skill, processedContent), nil
}

// ExecuteWithInjection loads a skill by name and returns a brief confirmation along
// with injection messages containing the full skill content. The injection messages
// will be merged into the conversation history so the model sees the full instructions
// as user context.
func (t *SkillTool) ExecuteWithInjection(name, args string) (string, []InjectionMessage, error) {
	if name == "" {
		return "", nil, fmt.Errorf("\"skill\" is required")
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
		return "", nil, fmt.Errorf("Skill not found: %q. Available skills: %s", name, available)
	}

	processedContent := SubstituteVariables(skill.Content, args, skill.DirPath)
	fullContent := BuildSkillInjectionContent(skill, processedContent)
	brief := fmt.Sprintf("Skill loaded: %s", skill.Metadata.Name)
	injectionMsg := InjectionMessage{
		Role:    "user",
		Content: fullContent,
	}
	return brief, []InjectionMessage{injectionMsg}, nil
}

// InjectionMessage represents a message to be injected into the conversation
// history alongside tool results.
type InjectionMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

// BuildSkillInjectionContent formats a skill's metadata and processed content
// into a full instruction string for injection into the conversation.
// It includes allowed_tools hints and fork mode markers.
func BuildSkillInjectionContent(skill *SkillInfo, content string) string {
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

	if len(skill.Metadata.AllowedTools) > 0 {
		parts = append(parts, "")
		parts = append(parts, fmt.Sprintf("Note: When following this skill's instructions, only use these tools: %s", strings.Join(skill.Metadata.AllowedTools, ", ")))
	}

	if skill.Metadata.Context == "fork" {
		parts = append(parts, "")
		parts = append(parts, "This skill should be executed in a fork context.")
	}

	return strings.Join(parts, "\n")
}
