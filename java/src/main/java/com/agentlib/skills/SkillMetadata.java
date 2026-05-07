package com.agentlib.skills;

import java.util.List;

/**
 * Metadata parsed from YAML frontmatter in SKILL.md files.
 */
public record SkillMetadata(
    String name,
    String description,
    String whenToUse,
    List<String> allowedTools,
    String model,
    String context,
    String version
) {

    private static final List<String> VALID_CONTEXTS = List.of("inline", "fork");

    public SkillMetadata {
        if (name != null && name.isBlank()) {
            name = null;
        }
        if (context != null && !VALID_CONTEXTS.contains(context)) {
            context = null;
        }
        if (allowedTools != null && allowedTools.isEmpty()) {
            allowedTools = null;
        }
    }
}
