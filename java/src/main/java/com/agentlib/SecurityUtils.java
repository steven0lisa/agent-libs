package com.agentlib;

import java.nio.file.Path;
import java.util.List;

/**
 * Security utilities for path validation and command filtering.
 */
public final class SecurityUtils {

    private SecurityUtils() {
        // utility class
    }

    /**
     * Resolve a path and ensure it stays within the working directory.
     *
     * @param filePath the relative or absolute file path
     * @param workDir  the working directory
     * @return the resolved path
     * @throws AgentException.SecurityException if the path escapes the working directory
     */
    public static Path resolveSafePath(String filePath, Path workDir) {
        Path work = workDir.toAbsolutePath().normalize();
        Path target = work.resolve(filePath).normalize();

        // Check if the resolved path is within the working directory
        if (!target.startsWith(work)) {
            throw new AgentException.SecurityException(
                "Path '" + filePath + "' escapes working directory '" + work + "'"
            );
        }

        return target;
    }

    /**
     * Check if text passes the whitelist/blacklist policy.
     * Whitelist is checked first; if matched, allow.
     * Then blacklist is checked; if matched, deny.
     *
     * @param text          the text to check
     * @param whitelist     whitelist patterns
     * @param blacklist     blacklist patterns
     * @param defaultAllow  default action when no pattern matches
     * @return SecurityCheckResult with allowed flag and reason
     */
    public static SecurityCheckResult checkSecurityPolicy(
        String text,
        List<Pattern> whitelist,
        List<Pattern> blacklist,
        boolean defaultAllow
    ) {
        // Check whitelist first - if matched, allow
        for (Pattern pattern : whitelist) {
            if (pattern.matches(text)) {
                return new SecurityCheckResult(true, "Matched whitelist pattern: " + pattern.pattern());
            }
        }

        // Check blacklist - if matched, deny
        for (Pattern pattern : blacklist) {
            if (pattern.matches(text)) {
                return new SecurityCheckResult(false, "Matched blacklist pattern: " + pattern.pattern());
            }
        }

        // Default action
        if (defaultAllow) {
            return new SecurityCheckResult(true, "Default allow");
        }
        return new SecurityCheckResult(false, "Default deny");
    }

    /**
     * Result of a security policy check.
     */
    public record SecurityCheckResult(boolean allowed, String reason) {}
}
