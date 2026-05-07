package agentlib

import (
	"testing"
	"time"
)

func TestDefaultConfig(t *testing.T) {
	cfg := DefaultConfig()

	if cfg.BaseURL != "https://api.anthropic.com" {
		t.Errorf("expected BaseURL to be https://api.anthropic.com, got %s", cfg.BaseURL)
	}
	if cfg.APIKey != "" {
		t.Errorf("expected APIKey to be empty (not read from env), got %s", cfg.APIKey)
	}
	if cfg.Model != "claude-sonnet-4-6" {
		t.Errorf("expected Model to be claude-sonnet-4-6, got %s", cfg.Model)
	}
	if cfg.MaxTokens != 8192 {
		t.Errorf("expected MaxTokens to be 8192, got %d", cfg.MaxTokens)
	}
	if cfg.MaxTurns != 100 {
		t.Errorf("expected MaxTurns to be 100, got %d", cfg.MaxTurns)
	}
	if cfg.Timeout != 2*time.Minute {
		t.Errorf("expected Timeout to be 2m, got %v", cfg.Timeout)
	}
	if !cfg.Stream {
		t.Error("expected Stream to be true")
	}
	if cfg.OutputFormat != OutputFormatText {
		t.Errorf("expected OutputFormat to be text, got %s", cfg.OutputFormat)
	}
	if cfg.WorkDir == "" {
		t.Error("expected WorkDir to be set")
	}
}

func TestExplicitConfig(t *testing.T) {
	cfg := Config{
		APIKey:  "test-key-123",
		BaseURL: "https://custom.api.com",
		Model:   "claude-opus-4",
	}

	if cfg.APIKey != "test-key-123" {
		t.Errorf("expected APIKey to be test-key-123, got %s", cfg.APIKey)
	}
	if cfg.BaseURL != "https://custom.api.com" {
		t.Errorf("expected BaseURL to be https://custom.api.com, got %s", cfg.BaseURL)
	}
	if cfg.Model != "claude-opus-4" {
		t.Errorf("expected Model to be claude-opus-4, got %s", cfg.Model)
	}
}

func TestPatternMatchesWildcard(t *testing.T) {
	tests := []struct {
		pattern string
		text    string
		want    bool
	}{
		{"ls*", "ls -la", true},
		{"ls*", "ls /tmp", true},
		{"ls*", "cat file.txt", false},
		{"rm*", "rm -rf /", true},
		{"rm*", "ls -la", false},
		{"*secret*", "mysecretfile", true},
		{"*secret*", "public", false},
		{"grep", "grep hello", true},
		{"grep", "ag hello", false},
	}

	for _, tt := range tests {
		p := Pattern{Pattern: tt.pattern, Type: "wildcard"}
		got := p.Matches(tt.text)
		if got != tt.want {
			t.Errorf("Pattern(%q).Matches(%q) = %v, want %v", tt.pattern, tt.text, got, tt.want)
		}
	}
}

func TestPatternMatchesRegex(t *testing.T) {
	tests := []struct {
		pattern string
		text    string
		want    bool
	}{
		{"^ls", "ls -la", true},
		{"^ls", "cat file", false},
		{"rm$", "rm", true},
		{"rm$", "rm -rf", false},
		{"exact", "exact", true},
		{"exact", "notexact", true}, // substring match for regex type with simple pattern
	}

	for _, tt := range tests {
		p := Pattern{Pattern: tt.pattern, Type: "regex"}
		got := p.Matches(tt.text)
		if got != tt.want {
			t.Errorf("Pattern(%q).Matches(%q) = %v, want %v", tt.pattern, tt.text, got, tt.want)
		}
	}
}

func TestPatternString(t *testing.T) {
	p1 := Pattern{Pattern: "test", Type: "wildcard"}
	if p1.String() != "wildcard(test)" {
		t.Errorf("expected String() to be wildcard(test), got %s", p1.String())
	}

	p2 := Pattern{Pattern: "test", Type: ""}
	if p2.String() != "test" {
		t.Errorf("expected String() to be test, got %s", p2.String())
	}
}

func TestOutputFormat(t *testing.T) {
	if OutputFormatText != "text" {
		t.Errorf("expected OutputFormatText to be 'text', got %s", OutputFormatText)
	}
	if OutputFormatJSON != "json" {
		t.Errorf("expected OutputFormatJSON to be 'json', got %s", OutputFormatJSON)
	}
}
