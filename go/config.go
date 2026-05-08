package agentlib

import (
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"
)

// OutputFormat defines the output format for agent responses.
type OutputFormat string

const (
	OutputFormatText OutputFormat = "text"
	OutputFormatJSON OutputFormat = "json"
)

// Pattern represents a whitelist or blacklist pattern.
// Supports wildcard and regex matching.
type Pattern struct {
	Pattern string `json:"pattern"`
	Type    string `json:"type"` // "wildcard" or "regex"
}

// Matches checks if the given text matches this pattern.
func (p Pattern) Matches(text string) bool {
	switch p.Type {
	case "regex":
		matched, err := regexp.MatchString(p.Pattern, text)
		return err == nil && matched
	case "wildcard", "":
		return wildcardMatch(p.Pattern, text)
	default:
		return false
	}
}

// wildcardMatch implements simple wildcard matching with * and ? support.
func wildcardMatch(pattern, text string) bool {
	// Empty pattern matches empty text
	if pattern == "" {
		return text == ""
	}

	// If pattern has no wildcards, do substring match
	if !strings.Contains(pattern, "*") && !strings.Contains(pattern, "?") {
		return strings.Contains(text, pattern)
	}

	// Use filepath.Match for standard wildcard matching
	matched, err := filepath.Match(pattern, text)
	if err == nil && matched {
		return true
	}

	// Also check if pattern matches as a prefix/suffix
	if strings.HasSuffix(pattern, "*") {
		prefix := pattern[:len(pattern)-1]
		if strings.HasPrefix(text, prefix) {
			return true
		}
	}
	if strings.HasPrefix(pattern, "*") {
		suffix := pattern[1:]
		if strings.HasSuffix(text, suffix) {
			return true
		}
	}
	if strings.HasPrefix(pattern, "*") && strings.HasSuffix(pattern, "*") {
		mid := pattern[1 : len(pattern)-1]
		if strings.Contains(text, mid) {
			return true
		}
	}

	// Fallback to substring match
	return strings.Contains(text, pattern)
}

// String returns a string representation of the pattern.
func (p Pattern) String() string {
	if p.Type != "" {
		return fmt.Sprintf("%s(%s)", p.Type, p.Pattern)
	}
	return p.Pattern
}

// Config holds the configuration for an Agent.
type Config struct {
	BaseURL          string        // Default: "https://api.anthropic.com"
	APIKey           string
	Model            string        // Default: "claude-sonnet-4-6"
	WorkDir          string        // Default: os.Getwd()
	MaxTokens        int           // Default: 8192
	MaxTurns         int           // Default: 100
	MaxDuration      time.Duration // Default: 0 (no limit)
	SystemPrompt     string        // Optional custom system prompt
	Timeout          time.Duration // Default: 2m
	Stream           bool          // Default: true
	HTTPClient       *http.Client  // Optional custom HTTP client (internal use)
	OutputFormat     OutputFormat  // Default: OutputFormatText
	Callback         func(Event)   // Optional per-step event callback
	EnableSubagent        bool          // Default: false
	SubagentMaxTurns      int           // Default: 50
	EnableSkills          bool          // Default: false
	SkillsDir             string        // Default: $HOME/.claude/skills
	IncludeProjectSkills  bool          // Default: false
	SkillsProjectDir      string        // Default: os.Getwd()
	BashWhitelist         []Pattern
	BashBlacklist         []Pattern
	CurlWhitelist         []Pattern
	CurlBlacklist         []Pattern
	AutoCompact           bool     // Default: true
	ContextWindowSize     int      // Default: 200000
	AutoCompactThresholdPct float64 // Default: 0.8
	AllowedReadDirs       []string
	AllowedWriteDirs      []string
}


// DefaultConfig returns a Config with sensible defaults.
// APIKey, BaseURL and Model must be set explicitly by the caller.
// The library does not read environment variables, to support
// multi-tenant scenarios with different credentials per instance.
func DefaultConfig() Config {
	wd, _ := os.Getwd()
	return Config{
		BaseURL:          "https://api.anthropic.com",
		APIKey:           "",
		Model:            "claude-sonnet-4-6",
		WorkDir:          wd,
		MaxTokens:        8192,
		MaxTurns:         100,
		MaxDuration:      0,
		Timeout:          2 * time.Minute,
		Stream:           true,
		OutputFormat:     OutputFormatText,
		EnableSubagent:        false,
		SubagentMaxTurns:      50,
		EnableSkills:          false,
		AutoCompact:            true,
		ContextWindowSize:      200000,
		AutoCompactThresholdPct: 0.8,
	}
}
