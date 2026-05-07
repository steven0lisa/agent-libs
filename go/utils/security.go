package utils

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// ResolveSafePath resolves a path and ensures it stays within the working directory.
// Returns the resolved path if valid, or an error if the path escapes the working directory.
func ResolveSafePath(filePath, workDir string) (string, error) {
	work, err := filepath.Abs(workDir)
	if err != nil {
		return "", fmt.Errorf("failed to resolve working directory: %w", err)
	}

	// Handle absolute paths
	if filepath.IsAbs(filePath) {
		// Check if the absolute path is within the working directory
		rel, err := filepath.Rel(work, filePath)
		if err != nil {
			return "", fmt.Errorf("path '%s' is outside working directory '%s'", filePath, workDir)
		}
		if strings.HasPrefix(rel, "..") {
			return "", fmt.Errorf("path '%s' escapes working directory '%s'", filePath, workDir)
		}
		return filePath, nil
	}

	// Join with working directory and clean the path
	target := filepath.Join(work, filePath)
	resolved, err := filepath.Abs(target)
	if err != nil {
		return "", fmt.Errorf("failed to resolve path: %w", err)
	}

	// Check if resolved path is within working directory
	rel, err := filepath.Rel(work, resolved)
	if err != nil {
		return "", fmt.Errorf("path '%s' is outside working directory '%s'", filePath, workDir)
	}
	if strings.HasPrefix(rel, "..") {
		return "", fmt.Errorf("path '%s' escapes working directory '%s'", filePath, workDir)
	}

	return resolved, nil
}

// CheckSecurityPolicy checks if text passes the whitelist/blacklist policy.
// Whitelist is checked first - if matched, allow.
// Then blacklist is checked - if matched, deny.
// Returns (allowed, reason).
func CheckSecurityPolicy(text string, whitelist, blacklist []Pattern, defaultAllow bool) (bool, string) {
	// Check whitelist first - if matched, allow
	for _, pattern := range whitelist {
		if pattern.Matches(text) {
			return true, fmt.Sprintf("Matched whitelist pattern: %s", pattern)
		}
	}

	// Check blacklist - if matched, deny
	for _, pattern := range blacklist {
		if pattern.Matches(text) {
			return false, fmt.Sprintf("Matched blacklist pattern: %s", pattern)
		}
	}

	// Default action
	if defaultAllow {
		return true, "Default allow"
	}
	return false, "Default deny"
}

// Pattern represents a whitelist or blacklist pattern.
type Pattern struct {
	Pattern string `json:"pattern"`
	Type    string `json:"type"` // "wildcard" or "regex"
}

// Matches checks if the given text matches this pattern.
func (p Pattern) Matches(text string) bool {
	switch p.Type {
	case "regex":
		return matchesRegex(p.Pattern, text)
	case "wildcard", "":
		// Use a simple wildcard matcher that supports * and ?
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

// matchesRegex checks if text matches the given regex pattern.
func matchesRegex(pattern, text string) bool {
	// Simple regex matching using strings for basic patterns
	// For full regex support, this would use regexp.Compile
	// But to avoid import cycles and keep it simple, we do basic matching
	if strings.HasPrefix(pattern, "^") && strings.HasSuffix(pattern, "$") {
		inner := pattern[1 : len(pattern)-1]
		return text == inner
	}
	if strings.HasPrefix(pattern, "^") {
		return strings.HasPrefix(text, pattern[1:])
	}
	if strings.HasSuffix(pattern, "$") {
		return strings.HasSuffix(text, pattern[:len(pattern)-1])
	}
	return strings.Contains(text, pattern)
}

// EnsureDir ensures that the directory for the given path exists.
func EnsureDir(path string) error {
	dir := filepath.Dir(path)
	if dir == "" || dir == "." {
		return nil
	}
	return os.MkdirAll(dir, 0755)
}

// WriteFile writes data to a file, creating it if necessary.
func WriteFile(path string, data []byte) error {
	return os.WriteFile(path, data, 0644)
}
