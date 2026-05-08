package tools

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/steven0lisa/agent-libs/go"
	"github.com/steven0lisa/agent-libs/go/utils"
)

// GlobTool finds files matching a glob pattern.
type GlobTool struct {
	agentlib.BaseTool
}

// Name returns the tool name.
func (g *GlobTool) Name() string { return "glob" }

// Description returns the tool description.
func (g *GlobTool) Description() string {
	return "Find files matching a glob pattern."
}

// IsReadOnly returns true since this tool only reads the file system.
func (g *GlobTool) IsReadOnly() bool { return true }

// InputSchema returns the JSON schema for the tool input.
func (g *GlobTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"pattern": map[string]any{
				"type":        "string",
				"description": "The glob pattern to match files against (e.g. '**/*.go', 'src/*.ts')",
			},
			"path": map[string]any{
				"type":        "string",
				"description": "The directory to search in (defaults to working directory)",
			},
		},
		"required": []string{"pattern"},
	}
}

// Call executes the glob tool.
func (g *GlobTool) Call(ctx context.Context, input map[string]any, toolCtx agentlib.ToolContext) (agentlib.ToolResult, error) {
	pattern, ok := input["pattern"].(string)
	if !ok || pattern == "" {
		return agentlib.Error("pattern is required"), nil
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

	// Check if resolved path exists
	info, err := os.Stat(resolved)
	if err != nil {
		return agentlib.Error(fmt.Sprintf("Path does not exist: %s", resolved)), nil
	}
	if !info.IsDir() {
		return agentlib.Error(fmt.Sprintf("Path is not a directory: %s", resolved)), nil
	}

	// Parse the glob pattern
	var results []string

	// Determine if we need recursive matching
	hasDoubleStar := strings.Contains(pattern, "**")

	// Directories to skip
	skipDirs := map[string]bool{
		".git":         true,
		"node_modules": true,
		"__pycache__":  true,
		".svn":         true,
		".hg":          true,
	}

	err = filepath.WalkDir(resolved, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return nil // skip entries we can't access
		}

		// Skip certain directories
		if d.IsDir() {
			if skipDirs[d.Name()] {
				return filepath.SkipDir
			}
			return nil
		}

		// Get relative path for matching
		relPath, err := filepath.Rel(resolved, path)
		if err != nil {
			return nil
		}

		// Convert to forward slashes for consistent matching
		relPath = filepath.ToSlash(relPath)

		// Match against the pattern
		if globMatch(pattern, relPath, hasDoubleStar) {
			// Return path relative to workdir for display
			displayPath, err := filepath.Rel(toolCtx.WorkDir, path)
			if err != nil {
				displayPath = path
			}
			results = append(results, displayPath)
		}

		return nil
	})

	if err != nil {
		return agentlib.Error(fmt.Sprintf("Error walking directory: %v", err)), nil
	}

	if len(results) == 0 {
		return agentlib.Success("No files matched the pattern."), nil
	}

	// Sort results for consistent output
	sort.Strings(results)

	result := strings.Join(results, "\n")
	result += fmt.Sprintf("\n\n(%d files found)", len(results))

	return agentlib.Success(result), nil
}

// globMatch matches a file path against a glob pattern.
// Supports:
//   - ** for matching any number of directories
//   - * for matching within a single path segment
//   - ? for matching a single character
func globMatch(pattern, path string, hasDoubleStar bool) bool {
	// Normalize pattern to forward slashes
	pattern = filepath.ToSlash(pattern)

	if hasDoubleStar {
		return matchDoubleStar(pattern, path)
	}

	// Simple single-segment matching
	// Split pattern by / and check each segment
	patternParts := strings.Split(pattern, "/")
	pathParts := strings.Split(path, "/")

	return matchSegments(patternParts, pathParts, 0, 0)
}

// matchDoubleStar handles patterns containing **.
func matchDoubleStar(pattern, path string) bool {
	// Split pattern on "**"
	parts := strings.Split(pattern, "**")
	pathParts := strings.Split(path, "/")

	if len(parts) == 1 {
		// No double star, treat as simple glob
		return matchSegments(strings.Split(pattern, "/"), pathParts, 0, 0)
	}

	// For each ** in the pattern, it can match zero or more path segments
	return matchDoubleStarRecursive(parts, pathParts, 0, 0)
}

// matchDoubleStarRecursive recursively matches ** patterns against path segments.
func matchDoubleStarRecursive(patternParts []string, pathParts []string, pi, pathi int) bool {
	// Base case: all pattern parts consumed
	if pi >= len(patternParts) {
		return pathi >= len(pathParts)
	}

	currentPattern := strings.Trim(strings.Trim(patternParts[pi], "/"), " ")

	// Empty pattern part (from leading/trailing **)
	if currentPattern == "" {
		if pi == len(patternParts)-1 {
			// Trailing ** matches everything remaining
			return true
		}
		return matchDoubleStarRecursive(patternParts, pathParts, pi+1, pathi)
	}

	segments := strings.Split(currentPattern, "/")

	// Try to match the current segments starting from each position in path
	for start := pathi; start <= len(pathParts); start++ {
		if matchSegmentsFrom(segments, pathParts, start) {
			nextPathi := start + len(segments)
			if matchDoubleStarRecursive(patternParts, pathParts, pi+1, nextPathi) {
				return true
			}
		}
	}

	return false
}

// matchSegmentsFrom checks if segments match pathParts starting from the given position.
func matchSegmentsFrom(segments, pathParts []string, start int) bool {
	if start+len(segments) > len(pathParts) {
		return false
	}
	for i, seg := range segments {
		if !singleSegmentMatch(seg, pathParts[start+i]) {
			return false
		}
	}
	return true
}

// matchSegments matches pattern segments against path segments.
func matchSegments(patternParts, pathParts []string, pi, pathi int) bool {
	// Both consumed
	if pi >= len(patternParts) && pathi >= len(pathParts) {
		return true
	}
	// One consumed but not the other
	if pi >= len(patternParts) || pathi >= len(pathParts) {
		return false
	}

	if singleSegmentMatch(patternParts[pi], pathParts[pathi]) {
		return matchSegments(patternParts, pathParts, pi+1, pathi+1)
	}

	return false
}

// singleSegmentMatch matches a single path segment against a pattern segment.
func singleSegmentMatch(pattern, name string) bool {
	// Use filepath.Match for standard glob (* and ?)
	matched, err := filepath.Match(pattern, name)
	if err == nil && matched {
		return true
	}

	// Exact match
	if pattern == name {
		return true
	}

	return false
}
