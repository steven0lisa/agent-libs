package com.agentlib;

/**
 * Exception thrown by Agent operations.
 */
public class AgentException extends RuntimeException {

    public AgentException(String message) {
        super(message);
    }

    public AgentException(String message, Throwable cause) {
        super(message, cause);
    }

    /**
     * Exception thrown when a tool is not found.
     */
    public static class ToolNotFoundException extends AgentException {
        public ToolNotFoundException(String toolName) {
            super("Tool not found: " + toolName);
        }
    }

    /**
     * Exception thrown when a security policy violation is detected.
     */
    public static class SecurityException extends AgentException {
        public SecurityException(String message) {
            super("Security violation: " + message);
        }
    }

    /**
     * Exception thrown when the maximum number of turns is reached.
     */
    public static class MaxTurnsExceededException extends AgentException {
        public MaxTurnsExceededException(int maxTurns) {
            super("Maximum turns exceeded: " + maxTurns);
        }
    }

    /**
     * Exception thrown when the maximum duration is exceeded.
     */
    public static class MaxDurationExceededException extends AgentException {
        public MaxDurationExceededException(long maxDurationMs) {
            super("Maximum duration exceeded: " + maxDurationMs + "ms");
        }
    }

    /**
     * Exception thrown when an API call fails.
     */
    public static class ApiException extends AgentException {
        public ApiException(String message) {
            super("API error: " + message);
        }

        public ApiException(String message, Throwable cause) {
            super("API error: " + message, cause);
        }
    }
}
