package tools

import (
	"bufio"
	"context"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

// GrepTool searches for patterns in files using regular expressions.
type GrepTool struct {
	agentlib.BaseTool
	Whitelist []utils.Pattern
	Blacklist []utils.Pattern
}

// NewGrepTool creates a new GrepTool with the given whitelist and blacklist.
func NewGrepTool(whitelist, blacklist []utils.Pattern) *GrepTool {
	return &GrepTool{
		Whitelist: whitelist,
		Blacklist: blacklist,
	}
}

// Name returns the tool name.
func (g *GrepTool) Name() string { return "grep" }

// Description returns the tool description.
func (g *GrepTool) Description() string {
	return "Search for patterns in files using regular expressions."
}

// IsReadOnly returns true since this tool only reads files.
func (g *GrepTool) IsReadOnly() bool { return true }

// InputSchema returns the JSON schema for the tool input.
func (g *GrepTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"pattern": map[string]any{
				"type":        "string",
				"description": "The regular expression pattern to search for",
			},
			"path": map[string]any{
				"type":        "string",
				"description": "The directory or file path to search in (defaults to working directory)",
			},
			"include": map[string]any{
				"type":        "string",
				"description": "File glob pattern to include (e.g. '*.go', '*.{js,ts}')",
			},
			"ignore_case": map[string]any{
				"type":        "boolean",
				"description": "Whether to perform a case-insensitive search",
				"default":     false,
			},
		},
		"required": []string{"pattern"},
	}
}

// Call executes the grep tool.
func (g *GrepTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	pattern, ok := input["pattern"].(string)
	if !ok || pattern == "" {
		return agentlib.Error("pattern is required"), nil
	}

	// Compile the regex pattern
	ignoreCase := agentlib.GetBool(input, "ignore_case", false)
	if ignoreCase {
		pattern = "(?i)" + pattern
	}
	re, err := regexp.Compile(pattern)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("Invalid regex pattern: %v", err)), nil
	}

	// Resolve search path
	searchPath := agentlib.GetString(input, "path", "")
	if searchPath == "" {
		searchPath = toolCtx.WorkDir
	}

	resolved, err := utils.ResolveSafePath(searchPath, toolCtx.WorkDir)
	if err != nil {
		return agentlib.Error(err.Error()), nil
	}

	// Get include pattern
	includePattern := agentlib.GetString(input, "include", "")

	// Directories to skip
	skipDirs := map[string]bool{
		".git":         true,
		"node_modules": true,
		"__pycache__":  true,
		".svn":         true,
		".hg":          true,
		"vendor":       true,
	}

	const maxMatches = 100
	var matches []string
	matchCount := 0

	// Walk the directory tree
	err = filepath.WalkDir(resolved, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return nil // skip files/dirs we can't access
		}

		// Skip directories
		if d.IsDir() {
			if skipDirs[d.Name()] {
				return filepath.SkipDir
			}
			return nil
		}

		// Check include pattern if specified
		if includePattern != "" && !matchIncludePattern(d.Name(), includePattern) {
			return nil
		}

		// Open and scan the file
		file, err := os.Open(path)
		if err != nil {
			return nil // skip files we can't open
		}
		defer file.Close()

		// Get relative path for display
		relPath, err := filepath.Rel(toolCtx.WorkDir, path)
		if err != nil {
			relPath = path
		}

		scanner := bufio.NewScanner(file)
		lineNum := 0
		for scanner.Scan() {
			lineNum++
			line := scanner.Text()
			if re.MatchString(line) {
				matches = append(matches, fmt.Sprintf("%s:%d: %s", relPath, lineNum, strings.TrimSpace(line)))
				matchCount++
				if matchCount >= maxMatches {
					return fmt.Errorf("max matches reached")
				}
			}
		}

		return nil
	})

	if err != nil && matchCount < maxMatches {
		// Only report real errors, not our max-matches sentinel
		if err.Error() != "max matches reached" {
			return agentlib.Error(fmt.Sprintf("Error walking directory: %v", err)), nil
		}
	}

	if len(matches) == 0 {
		return agentlib.Success("No matches found."), nil
	}

	result := strings.Join(matches, "\n")
	if matchCount >= maxMatches {
		result += fmt.Sprintf("\n\n(Results limited to %d matches)", maxMatches)
	}

	return agentlib.Success(result), nil
}

// matchIncludePattern checks if a filename matches the include glob pattern.
// Supports patterns like "*.go", "*.js", "*.{js,ts}", etc.
func matchIncludePattern(name, pattern string) bool {
	// Handle brace expansion patterns like *.{js,ts}
	if strings.Contains(pattern, "{") && strings.Contains(pattern, "}") {
		start := strings.Index(pattern, "{")
		end := strings.Index(pattern, "}")
		if start < end {
			prefix := pattern[:start]
			suffix := pattern[end+1:]
			alternatives := strings.Split(pattern[start+1:end], ",")
			for _, alt := range alternatives {
				expanded := prefix + strings.TrimSpace(alt) + suffix
				if matchGlob(name, expanded) {
					return true
				}
			}
			return false
		}
	}

	return matchGlob(name, pattern)
}

// matchGlob performs simple glob matching for file names.
func matchGlob(name, pattern string) bool {
	matched, err := filepath.Match(pattern, name)
	if err == nil && matched {
		return true
	}
	// Fallback: check if pattern appears in the name
	return strings.Contains(name, strings.ReplaceAll(pattern, "*", ""))
}
