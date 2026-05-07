package skills

import (
	"os"
	"regexp"
)

// SubstituteVariables replaces variable placeholders in skill content.
//
// Supported variables:
//   - $ARGUMENTS           replaced with the user-provided arguments string
//   - ${CLAUDE_SKILL_DIR}  replaced with the skill's directory path
//   - ${ENV:VAR_NAME}      replaced with the value of the environment variable VAR_NAME
//
// This is a pure function with no side effects.
func SubstituteVariables(content string, args string, skillDir string) string {
	result := content

	if args != "" {
		re := regexp.MustCompile(`\$ARGUMENTS`)
		result = re.ReplaceAllString(result, args)
	}

	if skillDir != "" {
		re := regexp.MustCompile(`\$\{CLAUDE_SKILL_DIR\}`)
		result = re.ReplaceAllString(result, skillDir)
	}

	// Replace ${ENV:VAR_NAME} with the corresponding environment variable value.
	re := regexp.MustCompile(`\$\{ENV:([^}]+)\}`)
	result = re.ReplaceAllStringFunc(result, func(match string) string {
		submatch := re.FindStringSubmatch(match)
		if len(submatch) > 1 {
			val := os.Getenv(submatch[1])
			return val
		}
		return match
	})

	return result
}
