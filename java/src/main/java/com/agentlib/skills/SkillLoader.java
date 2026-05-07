package com.agentlib.skills;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.stream.Stream;

/**
 * Discovers and caches SKILL.md files from configured directories.
 *
 * <p>Scans subdirectories of the skills directory for {@code SKILL.md} files,
 * parses their YAML frontmatter, and returns {@link SkillInfo} records.
 * Results are cached for the lifetime of the loader; call {@link #clearCache()}
 * to invalidate and force a re-scan.
 */
public class SkillLoader {

    private List<SkillInfo> cache;
    private final Path skillsDir;
    private final boolean includeProjectSkills;
    private final Path projectDir;

    /**
     * Create a SkillLoader with the given options.
     *
     * @param skillsDir            directory containing skill subdirectories (default: {@code ~/.claude/skills})
     * @param includeProjectSkills whether to also scan the project's {@code .claude/skills} directory
     * @param projectDir           project root directory (for project-level skills)
     */
    public SkillLoader(Path skillsDir, boolean includeProjectSkills, Path projectDir) {
        this.skillsDir = skillsDir != null
            ? skillsDir
            : Path.of(System.getProperty("user.home"), ".claude", "skills");
        this.includeProjectSkills = includeProjectSkills;
        this.projectDir = projectDir != null ? projectDir : Path.of("").toAbsolutePath();
        this.cache = null;
    }

    /**
     * Create a SkillLoader with default settings.
     */
    public SkillLoader() {
        this(null, false, null);
    }

    /**
     * Discover all skills from configured directories. Results are cached.
     *
     * @return list of discovered skills (never null)
     */
    public List<SkillInfo> discoverAll() {
        if (cache != null) {
            return cache;
        }

        List<SkillInfo> skills = new ArrayList<>();
        Set<String> scannedDirs = new HashSet<>();

        // User skills dir (higher priority — scanned first so duplicates are skipped)
        scanDirectory(skillsDir, skills, scannedDirs);

        // Project skills dir (optional)
        if (includeProjectSkills) {
            Path projectSkillsDir = projectDir.resolve(".claude").resolve("skills");
            if (!projectSkillsDir.equals(skillsDir)) {
                scanDirectory(projectSkillsDir, skills, scannedDirs);
            }
        }

        cache = Collections.unmodifiableList(skills);
        return cache;
    }

    /**
     * Find a skill by name. Returns empty if not found.
     *
     * @param name the skill name
     * @return an Optional containing the skill, or empty if not found
     */
    public Optional<SkillInfo> findByName(String name) {
        return discoverAll().stream()
            .filter(s -> name.equals(s.metadata().name()))
            .findFirst();
    }

    /**
     * Clear the cached skill list. The next call to {@link #discoverAll()}
     * will re-scan the directories.
     */
    public void clearCache() {
        this.cache = null;
    }

    private void scanDirectory(Path dir, List<SkillInfo> result, Set<String> scannedDirs) {
        if (!Files.isDirectory(dir)) {
            return;
        }

        try (Stream<Path> entries = Files.list(dir)) {
            entries.filter(Files::isDirectory)
                .forEach(skillDir -> {
                    String dirName = skillDir.getFileName().toString();
                    if (scannedDirs.contains(dirName)) {
                        return;
                    }

                    Path skillFile = skillDir.resolve("SKILL.md");
                    if (!Files.isRegularFile(skillFile)) {
                        return;
                    }

                    try {
                        var parsed = FrontmatterParser.parseSkillFile(skillFile);
                        String name = parsed.metadata().name();
                        if (name == null || name.isBlank()) {
                            name = dirName;
                        }

                        scannedDirs.add(dirName);
                        result.add(new SkillInfo(parsed.metadata(), parsed.content(),
                            skillFile.toAbsolutePath().toString(),
                            skillDir.toAbsolutePath().toString()));
                    } catch (IOException e) {
                        // Silently skip invalid skills
                    }
                });
        } catch (IOException e) {
            // Silently skip invalid directories
        }
    }
}
