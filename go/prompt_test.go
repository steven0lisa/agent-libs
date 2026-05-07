package agentlib

import (
	"strings"
	"testing"
)

func TestBuildSystemPrompt(t *testing.T) {
	tools := map[string]Tool{
		"read_file": &mockTool{
			name:        "read_file",
			description: "Read a file",
			readOnly:    true,
		},
	}

	prompt := BuildSystemPrompt(tools, "", false, 50, nil)

	if !strings.Contains(prompt, "read_file") {
		t.Error("expected prompt to contain tool name")
	}
	if !strings.Contains(prompt, "Read a file") {
		t.Error("expected prompt to contain tool description")
	}
	if !strings.Contains(prompt, "Available Tools") {
		t.Error("expected prompt to contain 'Available Tools' section")
	}
}

func TestBuildSystemPromptWithCustomPrompt(t *testing.T) {
	tools := map[string]Tool{}
	custom := "Be extra careful with rm commands"

	prompt := BuildSystemPrompt(tools, custom, false, 50, nil)

	if !strings.Contains(prompt, "Custom Instructions") {
		t.Error("expected prompt to contain 'Custom Instructions' section")
	}
	if !strings.Contains(prompt, custom) {
		t.Error("expected prompt to contain custom prompt text")
	}
}

func TestBuildSystemPromptWithSubagent(t *testing.T) {
	tools := map[string]Tool{}

	prompt := BuildSystemPrompt(tools, "", true, 30, nil)

	if !strings.Contains(prompt, "Subagent") {
		t.Error("expected prompt to contain subagent section")
	}
	if !strings.Contains(prompt, "30") {
		t.Error("expected prompt to contain subagent max turns")
	}
}

func TestBuildSystemPromptNoTools(t *testing.T) {
	tools := map[string]Tool{}

	prompt := BuildSystemPrompt(tools, "", false, 50, nil)

	if !strings.Contains(prompt, "Tool Use") {
		t.Error("expected prompt to contain default tool use instructions")
	}
	if strings.Contains(prompt, "Available Tools") {
		t.Error("expected prompt to not contain 'Available Tools' when no tools")
	}
}
