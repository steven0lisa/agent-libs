package com.agentlib.skills;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Minimal YAML frontmatter parser for SKILL.md files.
 * Parses {@code ---} delimited blocks with key:value pairs.
 * No external dependencies.
 */
public final class FrontmatterParser {

    private static final Pattern FRONTMATTER_PATTERN =
        Pattern.compile("^---\\n([\\s\\S]*?)\\n---\\n?", Pattern.MULTILINE);

    private static final Pattern KEY_VALUE_PATTERN =
        Pattern.compile("^(\\w[\\w_]*)\\s*:\\s*(.*)$", Pattern.MULTILINE);

    private FrontmatterParser() {
        // utility class
    }

    /**
     * Parse the YAML frontmatter block from a raw string and return metadata.
     *
     * @param raw the full file content (including frontmatter)
     * @return parsed metadata (fields may be null if not present)
     */
    public static SkillMetadata parseFrontmatter(String raw) {
        Matcher matcher = FRONTMATTER_PATTERN.matcher(raw);
        if (!matcher.find()) {
            return new SkillMetadata(null, null, null, null, null, null, null);
        }

        String yamlBlock = matcher.group(1);
        String name = null;
        String description = null;
        String whenToUse = null;
        List<String> allowedTools = null;
        String model = null;
        String context = null;
        String version = null;
        boolean userInvocable = true;

        Matcher kvMatcher = KEY_VALUE_PATTERN.matcher(yamlBlock);
        String currentKey = null;
        StringBuilder currentMultiline = null;

        while (kvMatcher.find()) {
            String key = kvMatcher.group(1);
            String value = kvMatcher.group(2).trim();

            // Check if this value starts a list on the same line
            if (value.isEmpty() || value.startsWith("#")) {
                // Value may be empty (list on next lines) or a comment
                currentKey = key;
                currentMultiline = new StringBuilder();
                continue;
            }

            currentKey = null;
            currentMultiline = null;

            // Handle inline list values (comma-separated after a dash)
            if (value.startsWith("- ")) {
                List<String> list = parseListValue(value.substring(2));
                if ("allowed_tools".equals(key)) {
                    allowedTools = list;
                }
                continue;
            }

            // Remove surrounding quotes
            value = stripQuotes(value);

            // Parse typed value
            Object parsed = parseScalar(value);

            switch (key) {
                case "name" -> name = (String) parsed;
                case "description" -> description = (String) parsed;
                case "when_to_use" -> whenToUse = (String) parsed;
                case "allowed_tools" -> {
                    if (parsed instanceof List<?> l) {
                        allowedTools = l.stream().map(Object::toString).toList();
                    } else {
                        allowedTools = List.of(parsed.toString());
                    }
                }
                case "model" -> model = (String) parsed;
                case "context" -> context = (String) parsed;
                case "version" -> version = (String) parsed;
                case "user_invocable" -> {
                    if (parsed instanceof Boolean b) {
                        userInvocable = b;
                    }
                }
                default -> { /* skip unknown keys */ }
            }
        }

        // Handle multiline list values (next lines starting with -)
        if (currentKey != null && currentMultiline != null) {
            String accum = currentMultiline.toString().trim();
            if (!accum.isEmpty()) {
                List<String> list = parseMultilineList(accum);
                if ("allowed_tools".equals(currentKey)) {
                    allowedTools = list;
                }
            }
        }

        return new SkillMetadata(name, description, whenToUse, allowedTools, model, context, version, userInvocable);
    }

    /**
     * Parse a SKILL.md file and return the metadata and content body.
     */
    public static SkillFileResult parseSkillFile(Path filePath) throws IOException {
        String raw = Files.readString(filePath);
        SkillMetadata metadata = parseFrontmatter(raw);

        // Remove frontmatter block, keep the rest
        String content = FRONTMATTER_PATTERN.matcher(raw).replaceFirst("").trim();

        return new SkillFileResult(metadata, content);
    }

    /**
     * Result of parsing a skill file.
     */
    public record SkillFileResult(SkillMetadata metadata, String content) {}

    // ---- Private helpers ----

    private static String stripQuotes(String value) {
        if ((value.startsWith("\"") && value.endsWith("\""))
            || (value.startsWith("'") && value.endsWith("'"))) {
            return value.substring(1, value.length() - 1);
        }
        return value;
    }

    private static Object parseScalar(String value) {
        if ("true".equals(value)) return true;
        if ("false".equals(value)) return false;
        try {
            if (!value.isEmpty()) {
                return Integer.parseInt(value);
            }
        } catch (NumberFormatException ignored) {
            // not an integer
        }
        return value;
    }

    private static List<String> parseListValue(String raw) {
        List<String> result = new ArrayList<>();
        for (String item : raw.split(",")) {
            String trimmed = item.trim();
            if (!trimmed.isEmpty()) {
                result.add(trimmed);
            }
        }
        return Collections.unmodifiableList(result);
    }

    private static List<String> parseMultilineList(String raw) {
        List<String> result = new ArrayList<>();
        for (String line : raw.split("\n")) {
            String trimmed = line.trim();
            if (trimmed.startsWith("- ")) {
                String value = trimmed.substring(2).trim();
                if (!value.isEmpty()) {
                    result.add(value);
                }
            }
        }
        return Collections.unmodifiableList(result);
    }

}
