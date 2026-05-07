package com.agentlib;

import java.nio.file.Path;
import java.util.List;

/**
 * Context passed to tools during execution.
 */
public record ToolContext(Path workDir, List<Message> messageHistory) {}
