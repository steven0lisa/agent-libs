package com.agentlib;

/**
 * A pattern for whitelist/blacklist matching.
 * Supports wildcard and regex patterns.
 */
public record Pattern(String pattern, String type) {

    public Pattern {
        if (type == null) {
            type = "wildcard";
        }
    }

    public Pattern(String pattern) {
        this(pattern, "wildcard");
    }

    /**
     * Check if the given text matches this pattern.
     */
    public boolean matches(String text) {
        if (text == null) {
            return false;
        }
        if ("wildcard".equals(type)) {
            return matchWildcard(text, pattern);
        } else if ("regex".equals(type)) {
            return text.matches(pattern);
        }
        return false;
    }

    private static boolean matchWildcard(String text, String pattern) {
        // Simple wildcard matching: * matches any sequence, ? matches single char
        int textLen = text.length();
        int patLen = pattern.length();
        int textIdx = 0;
        int patIdx = 0;
        int starIdx = -1;
        int matchIdx = 0;

        while (textIdx < textLen) {
            if (patIdx < patLen && (pattern.charAt(patIdx) == '?' || pattern.charAt(patIdx) == text.charAt(textIdx))) {
                textIdx++;
                patIdx++;
            } else if (patIdx < patLen && pattern.charAt(patIdx) == '*') {
                starIdx = patIdx;
                matchIdx = textIdx;
                patIdx++;
            } else if (starIdx != -1) {
                patIdx = starIdx + 1;
                matchIdx++;
                textIdx = matchIdx;
            } else {
                return false;
            }
        }

        while (patIdx < patLen && pattern.charAt(patIdx) == '*') {
            patIdx++;
        }

        return patIdx == patLen;
    }
}
