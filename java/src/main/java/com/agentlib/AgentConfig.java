package com.agentlib;

import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;

/**
 * Output format for agent responses.
 */
enum OutputFormat {
    TEXT("text"),
    JSON("json");

    private final String value;

    OutputFormat(String value) {
        this.value = value;
    }

    public String value() {
        return value;
    }
}

/**
 * Configuration for an Agent instance.
 */
public class AgentConfig {
    private final String baseUrl;
    private final String apiKey;
    private final String model;
    private final Path workDir;
    private final int maxTokens;
    private final int maxTurns;
    private final String systemPrompt;
    private final Duration timeout;
    private final boolean stream;
    private final List<Tool> customTools;
    private final OutputFormat outputFormat;
    private final long maxDurationMs;
    private final boolean enableSubagent;
    private final int subagentMaxTurns;
    private final List<Pattern> bashWhitelist;
    private final List<Pattern> bashBlacklist;
    private final List<Pattern> curlWhitelist;
    private final List<Pattern> curlBlacklist;
    private final boolean enableSkills;
    private final Path skillsDir;
    private final boolean includeProjectSkills;
    private final Path skillsProjectDir;
    private final boolean autoCompact;
    private final int contextWindowSize;
    private final double autoCompactThresholdPct;
    private final List<Path> allowedReadDirs;
    private final List<Path> allowedWriteDirs;

    private AgentConfig(Builder builder) {
        this.baseUrl = builder.baseUrl != null ? builder.baseUrl : "https://api.anthropic.com";
        this.apiKey = builder.apiKey;
        this.model = builder.model != null ? builder.model : "claude-sonnet-4-6";
        this.workDir = builder.workDir != null ? builder.workDir : Paths.get("").toAbsolutePath();
        this.maxTokens = builder.maxTokens > 0 ? builder.maxTokens : 8192;
        this.maxTurns = builder.maxTurns > 0 ? builder.maxTurns : 100;
        this.systemPrompt = builder.systemPrompt;
        this.timeout = builder.timeout != null ? builder.timeout : Duration.ofMinutes(2);
        this.stream = builder.stream;
        this.customTools = builder.customTools != null ? List.copyOf(builder.customTools) : List.of();
        this.outputFormat = builder.outputFormat != null ? builder.outputFormat : OutputFormat.TEXT;
        this.maxDurationMs = builder.maxDurationMs;
        this.enableSubagent = builder.enableSubagent;
        this.subagentMaxTurns = builder.subagentMaxTurns > 0 ? builder.subagentMaxTurns : 50;
        this.bashWhitelist = builder.bashWhitelist != null ? List.copyOf(builder.bashWhitelist) : List.of();
        this.bashBlacklist = builder.bashBlacklist != null ? List.copyOf(builder.bashBlacklist) : List.of();
        this.curlWhitelist = builder.curlWhitelist != null ? List.copyOf(builder.curlWhitelist) : List.of();
        this.curlBlacklist = builder.curlBlacklist != null ? List.copyOf(builder.curlBlacklist) : List.of();
        this.enableSkills = builder.enableSkills;
        this.skillsDir = builder.skillsDir;
        this.includeProjectSkills = builder.includeProjectSkills;
        this.skillsProjectDir = builder.skillsProjectDir;
        this.autoCompact = builder.autoCompact;
        this.contextWindowSize = builder.contextWindowSize > 0 ? builder.contextWindowSize : 200000;
        this.autoCompactThresholdPct = builder.autoCompactThresholdPct > 0 ? builder.autoCompactThresholdPct : 0.8;
        this.allowedReadDirs = builder.allowedReadDirs != null ? List.copyOf(builder.allowedReadDirs) : List.of();
        this.allowedWriteDirs = builder.allowedWriteDirs != null ? List.copyOf(builder.allowedWriteDirs) : List.of();
    }

    public String baseUrl() { return baseUrl; }
    public String apiKey() { return apiKey; }
    public String model() { return model; }
    public Path workDir() { return workDir; }
    public int maxTokens() { return maxTokens; }
    public int maxTurns() { return maxTurns; }
    public String systemPrompt() { return systemPrompt; }
    public Duration timeout() { return timeout; }
    public boolean stream() { return stream; }
    public List<Tool> customTools() { return customTools; }
    public OutputFormat outputFormat() { return outputFormat; }
    public long maxDurationMs() { return maxDurationMs; }
    public boolean enableSubagent() { return enableSubagent; }
    public int subagentMaxTurns() { return subagentMaxTurns; }
    public List<Pattern> bashWhitelist() { return bashWhitelist; }
    public List<Pattern> bashBlacklist() { return bashBlacklist; }
    public List<Pattern> curlWhitelist() { return curlWhitelist; }
    public List<Pattern> curlBlacklist() { return curlBlacklist; }
    public boolean enableSkills() { return enableSkills; }
    public Path skillsDir() { return skillsDir; }
    public boolean includeProjectSkills() { return includeProjectSkills; }
    public Path skillsProjectDir() { return skillsProjectDir; }
    public boolean autoCompact() { return autoCompact; }
    public int contextWindowSize() { return contextWindowSize; }
    public double autoCompactThresholdPct() { return autoCompactThresholdPct; }
    public List<Path> allowedReadDirs() { return allowedReadDirs; }
    public List<Path> allowedWriteDirs() { return allowedWriteDirs; }

    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for AgentConfig.
     */
    public static class Builder {
        private String baseUrl;
        private String apiKey;
        private String model;
        private Path workDir;
        private int maxTokens;
        private int maxTurns;
        private String systemPrompt;
        private Duration timeout;
        private boolean stream = true;
        private List<Tool> customTools;
        private OutputFormat outputFormat;
        private long maxDurationMs;
        private boolean enableSubagent;
        private int subagentMaxTurns;
        private List<Pattern> bashWhitelist;
        private List<Pattern> bashBlacklist;
        private List<Pattern> curlWhitelist;
        private List<Pattern> curlBlacklist;
        private boolean enableSkills;
        private Path skillsDir;
        private boolean includeProjectSkills;
        private Path skillsProjectDir;
        private boolean autoCompact = true;
        private int contextWindowSize;
        private double autoCompactThresholdPct;
        private List<Path> allowedReadDirs;
        private List<Path> allowedWriteDirs;

        public Builder baseUrl(String baseUrl) {
            this.baseUrl = baseUrl;
            return this;
        }

        public Builder apiKey(String apiKey) {
            this.apiKey = apiKey;
            return this;
        }

        public Builder model(String model) {
            this.model = model;
            return this;
        }

        public Builder workDir(Path workDir) {
            this.workDir = workDir;
            return this;
        }

        public Builder maxTokens(int maxTokens) {
            this.maxTokens = maxTokens;
            return this;
        }

        public Builder maxTurns(int maxTurns) {
            this.maxTurns = maxTurns;
            return this;
        }

        public Builder systemPrompt(String systemPrompt) {
            this.systemPrompt = systemPrompt;
            return this;
        }

        public Builder timeout(Duration timeout) {
            this.timeout = timeout;
            return this;
        }

        public Builder stream(boolean stream) {
            this.stream = stream;
            return this;
        }

        public Builder customTools(List<Tool> customTools) {
            this.customTools = customTools;
            return this;
        }

        public Builder outputFormat(OutputFormat outputFormat) {
            this.outputFormat = outputFormat;
            return this;
        }

        public Builder maxDurationMs(long maxDurationMs) {
            this.maxDurationMs = maxDurationMs;
            return this;
        }

        public Builder enableSubagent(boolean enableSubagent) {
            this.enableSubagent = enableSubagent;
            return this;
        }

        public Builder subagentMaxTurns(int subagentMaxTurns) {
            this.subagentMaxTurns = subagentMaxTurns;
            return this;
        }

        public Builder bashWhitelist(List<Pattern> bashWhitelist) {
            this.bashWhitelist = bashWhitelist;
            return this;
        }

        public Builder bashBlacklist(List<Pattern> bashBlacklist) {
            this.bashBlacklist = bashBlacklist;
            return this;
        }

        public Builder curlWhitelist(List<Pattern> curlWhitelist) {
            this.curlWhitelist = curlWhitelist;
            return this;
        }

        public Builder curlBlacklist(List<Pattern> curlBlacklist) {
            this.curlBlacklist = curlBlacklist;
            return this;
        }

        public Builder enableSkills(boolean enableSkills) {
            this.enableSkills = enableSkills;
            return this;
        }

        public Builder skillsDir(Path skillsDir) {
            this.skillsDir = skillsDir;
            return this;
        }

        public Builder includeProjectSkills(boolean includeProjectSkills) {
            this.includeProjectSkills = includeProjectSkills;
            return this;
        }

        public Builder skillsProjectDir(Path skillsProjectDir) {
            this.skillsProjectDir = skillsProjectDir;
            return this;
        }

        public Builder autoCompact(boolean autoCompact) {
            this.autoCompact = autoCompact;
            return this;
        }

        public Builder contextWindowSize(int contextWindowSize) {
            this.contextWindowSize = contextWindowSize;
            return this;
        }

        public Builder autoCompactThresholdPct(double autoCompactThresholdPct) {
            this.autoCompactThresholdPct = autoCompactThresholdPct;
            return this;
        }

        public Builder allowedReadDirs(List<Path> allowedReadDirs) {
            this.allowedReadDirs = allowedReadDirs;
            return this;
        }

        public Builder allowedWriteDirs(List<Path> allowedWriteDirs) {
            this.allowedWriteDirs = allowedWriteDirs;
            return this;
        }

        public AgentConfig build() {
            return new AgentConfig(this);
        }
    }
}
