package com.agentlib;

import java.nio.file.Path;
import java.util.List;

/**
 * Context passed to tools during execution.
 */
public record ToolContext(
    Path workDir,
    List<Message> messageHistory,
    List<Path> allowedReadDirs,
    List<Path> allowedWriteDirs
) {
    /**
     * Compatibility constructor without directory restrictions.
     */
    public ToolContext(Path workDir, List<Message> messageHistory) {
        this(workDir, messageHistory, List.of(), List.of());
    }
}
