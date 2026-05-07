package com.agentlib.skills;

import java.util.regex.Pattern;

/**
 * Variable substitution for skill content.
 *
 * <p>Supported variables:
 * <ul>
 *   <li>{@code $ARGUMENTS} — replaced with user-provided arguments</li>
 *   <li>{@code ${CLAUDE_SKILL_DIR}} — replaced with the skill's directory path</li>
 *   <li>{@code ${ENV:VAR_NAME}} — replaced with the value of the environment variable</li>
 * </ul>
 */
public final class VariableSubstitutor {

    private static final Pattern ARGUMENTS_PATTERN = Pattern.compile("\\$ARGUMENTS");
    private static final Pattern SKILL_DIR_PATTERN = Pattern.compile("\\$\\{CLAUDE_SKILL_DIR\\}");
    private static final Pattern ENV_PATTERN = Pattern.compile("\\$\\{ENV:([^}]+)\\}");

    private VariableSubstitutor() {
        // utility class
    }

    /**
     * Substitute variables in skill content.
     *
     * @param content  the raw skill content
     * @param args     user-provided arguments (may be null)
     * @param skillDir the skill directory path (may be null)
     * @return the content with variables substituted
     */
    public static String substitute(String content, String args, String skillDir) {
        String result = content;

        if (args != null) {
            result = ARGUMENTS_PATTERN.matcher(result)
                .replaceAll(java.util.regex.Matcher.quoteReplacement(args));
        }

        if (skillDir != null) {
            result = SKILL_DIR_PATTERN.matcher(result)
                .replaceAll(java.util.regex.Matcher.quoteReplacement(skillDir));
        }

        result = ENV_PATTERN.matcher(result).replaceAll(m -> {
            String envName = m.group(1);
            String envValue = System.getenv(envName);
            return envValue != null
                ? java.util.regex.Matcher.quoteReplacement(envValue)
                : "";
        });

        return result;
    }
}
