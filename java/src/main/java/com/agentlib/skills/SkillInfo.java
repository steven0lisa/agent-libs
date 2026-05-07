package com.agentlib.skills;

import java.nio.file.Path;

/**
 * Represents a discovered skill from a SKILL.md file on disk.
 */
public record SkillInfo(
    SkillMetadata metadata,
    String content,
    String filePath,
    String dirPath
) {}
