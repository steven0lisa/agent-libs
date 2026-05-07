package skills

import (
	"os"
	"strconv"
	"strings"
)

// parseFrontmatter extracts metadata from YAML frontmatter delimited by "---".
// It supports simple key: value pairs, booleans, numbers, quoted strings,
// and array values prefixed with "- " (single-line or multi-line).
func parseFrontmatter(raw string) Metadata {
	var meta Metadata
	lines := strings.Split(raw, "\n")
	currentKey := ""

	// Use a map to collect values during parsing, then assign to struct fields.
	fields := make(map[string]any)

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}

		// Array continuation line (starts with "- ")
		if strings.HasPrefix(trimmed, "- ") && currentKey != "" {
			values := parseArrayValues(trimmed[2:])
			existing, ok := fields[currentKey].([]string)
			if ok {
				fields[currentKey] = append(existing, values...)
			} else {
				fields[currentKey] = values
			}
			continue
		}

		colonIdx := strings.Index(trimmed, ":")
		if colonIdx == -1 {
			currentKey = ""
			continue
		}

		key := strings.TrimSpace(trimmed[:colonIdx])
		value := strings.TrimSpace(trimmed[colonIdx+1:])
		currentKey = key

		if value == "" {
			// Key with no value — array values may follow on subsequent lines.
			continue
		}

		// Single-line array: "allowed_tools: - tool1, tool2"
		if strings.HasPrefix(value, "- ") {
			fields[key] = parseArrayValues(value[2:])
			continue
		}

		fields[key] = parseScalarValue(value)
	}

	// Map parsed fields to the Metadata struct.
	if v, ok := fields["name"].(string); ok {
		meta.Name = v
	}
	if v, ok := fields["description"].(string); ok {
		meta.Description = v
	}
	if v, ok := fields["when_to_use"].(string); ok {
		meta.WhenToUse = v
	}
	if v, ok := fields["allowed_tools"]; ok {
		switch arr := v.(type) {
		case []string:
			// Split each element by comma to handle "tool1, tool2" patterns.
			var flat []string
			for _, s := range arr {
				flat = append(flat, strings.Split(s, ",")...)
			}
			for i := range flat {
				flat[i] = strings.TrimSpace(flat[i])
			}
			meta.AllowedTools = flat
		}
	}
	if v, ok := fields["model"].(string); ok {
		meta.Model = v
	}
	if v, ok := fields["context"].(string); ok {
		meta.Context = v
	}
	if v, ok := fields["version"].(string); ok {
		meta.Version = v
	}

	return meta
}

// parseScalarValue converts a string value into its typed representation,
// handling quoted strings, booleans, and numbers.
func parseScalarValue(value string) any {
	// Strip surrounding quotes
	if (strings.HasPrefix(value, "\"") && strings.HasSuffix(value, "\"")) ||
		(strings.HasPrefix(value, "'") && strings.HasSuffix(value, "'")) {
		return value[1 : len(value)-1]
	}

	// Try boolean
	if value == "true" {
		return true
	}
	if value == "false" {
		return false
	}

	// Try number
	if num, err := strconv.ParseFloat(value, 64); err == nil {
		if num == float64(int(num)) {
			return int(num)
		}
		return num
	}

	return value
}

// parseArrayValues splits a comma-separated string into trimmed parts.
func parseArrayValues(raw string) []string {
	parts := strings.Split(raw, ",")
	result := make([]string, 0, len(parts))
	for _, p := range parts {
		trimmed := strings.TrimSpace(p)
		if trimmed != "" {
			result = append(result, trimmed)
		}
	}
	return result
}

// ParseSkillFile reads a SKILL.md file, extracts its frontmatter, and returns
// the metadata along with the remaining content (after frontmatter is removed).
func ParseSkillFile(filePath string) (Metadata, string, error) {
	raw, err := os.ReadFile(filePath)
	if err != nil {
		return Metadata{}, "", err
	}

	content := string(raw)
	meta := parseFrontmatter(content)

	// Remove frontmatter: everything between the first "---\n" and the second "---\n".
	const delimiter = "---\n"
	firstIdx := strings.Index(content, delimiter)
	if firstIdx == -1 {
		// No frontmatter — return metadata as-is with full content.
		return meta, strings.TrimSpace(content), nil
	}

	rest := content[firstIdx+len(delimiter):]
	secondIdx := strings.Index(rest, delimiter)
	if secondIdx == -1 {
		// Malformed — only one delimiter, return content as-is.
		return meta, strings.TrimSpace(content), nil
	}

	body := rest[secondIdx+len(delimiter):]
	return meta, strings.TrimSpace(body), nil
}
