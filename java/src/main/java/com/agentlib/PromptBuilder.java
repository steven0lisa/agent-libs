package com.agentlib;

import com.agentlib.skills.SkillInfo;

import java.util.Collection;
import java.util.List;

/**
 * Builds the system prompt with tool descriptions.
 */
public final class PromptBuilder {

    private static final String DEFAULT_SYSTEM_PROMPT = """
        You are a helpful software engineering assistant. You have access to tools that let you interact with the file system and execute commands.

        ## Tool Use

        You will be provided with a set of tools. When you need to perform an action, use the appropriate tool by outputting a tool_use block.

        Tool use format:
        - Each tool has a name and input parameters defined by a JSON schema
        - To use a tool, output a content block with type "tool_use"
        - The tool_use block must include: name (tool name), id (unique identifier), input (parameters as JSON object)

        After using a tool, the user message will contain a tool_result block with the execution result.
        - If the result indicates success, continue with your analysis or next steps
        - If the result indicates an error, analyze the error and decide whether to retry, use a different approach, or ask the user for clarification

        ## File Operations

        When working with files:
        - Always read a file before editing it
        - When editing files, use update_file tool with old_string/new_string for precise changes
        - Use write_file to create new files
        - Use read_file to inspect file contents
        - All file operations are restricted to the working directory and its subdirectories

        ## Command Execution

        When executing commands via bash:
        - Prefer read-only commands for exploration (ls, grep, find, cat, etc.)
        - Be careful with destructive commands (rm, dd, etc.)
        - Always describe what the command does before executing it
        """;

    private static final String SUBAGENT_PROMPT = """

        ## Subagent Capability

        You can create subagents to handle independent tasks in parallel. A subagent is a separate agent instance that shares your context but operates independently.

        When to use subagents:
        - When you need to explore multiple approaches simultaneously
        - When a task can be cleanly decomposed into independent sub-tasks
        - When you want to parallelize read-only exploration

        How to create a subagent:
        - Use the `subagent` tool with a `task` describing what the subagent should do
        - The subagent will execute with its own tool budget (max turns: %d)
        - The subagent inherits your current context (files read, tool state, etc.)
        - Results are returned as tool_result when the subagent completes

        Important:
        - Subagents run independently and do not modify your state
        - File operations in subagents are still restricted to the working directory
        - Prefer subagents for exploration; keep modifications in the main agent
        """;

    private static final String SKILLS_SECTION_HEADER = """

        ## Available Skills

        You can load specialized skills to extend your capabilities for specific tasks.
        Use the `skill` tool to load a skill by name.

        """;

    private static final String SKILL_USAGE_HINT = """
        When you need to perform a task that matches a skill's purpose, load the skill
        first using the `skill` tool to get specialized instructions.

        """;

    private PromptBuilder() {
        // utility class
    }

    /**
     * Build the system prompt with tool descriptions.
     *
     * @param tools          the available tools
     * @param customPrompt   optional custom prompt to append
     * @param enableSubagent whether to include subagent instructions
     * @param subagentMaxTurns max turns for subagents
     * @return the complete system prompt
     */
    public static String build(
        Collection<Tool> tools,
        String customPrompt,
        boolean enableSubagent,
        int subagentMaxTurns
    ) {
        return build(tools, customPrompt, enableSubagent, subagentMaxTurns, List.of());
    }

    /**
     * Build the system prompt with tool descriptions and available skills.
     *
     * @param tools            the available tools
     * @param customPrompt     optional custom prompt to append
     * @param enableSubagent   whether to include subagent instructions
     * @param subagentMaxTurns max turns for subagents
     * @param skills           available skills to list (may be empty)
     * @return the complete system prompt
     */
    public static String build(
        Collection<Tool> tools,
        String customPrompt,
        boolean enableSubagent,
        int subagentMaxTurns,
        List<SkillInfo> skills
    ) {
        StringBuilder prompt = new StringBuilder(DEFAULT_SYSTEM_PROMPT);

        // Tool descriptions
        prompt.append("\n\n## Available Tools\n\n");
        for (Tool tool : tools) {
            prompt.append("### ").append(tool.name()).append("\n");
            prompt.append("Description: ").append(tool.description()).append("\n");
            prompt.append("Input Schema: ").append(tool.inputSchema().toString()).append("\n\n");
        }

        if (enableSubagent) {
            prompt.append(String.format(SUBAGENT_PROMPT, subagentMaxTurns));
        }

        // Skills section
        if (skills != null && !skills.isEmpty()) {
            prompt.append(SKILLS_SECTION_HEADER);
            for (SkillInfo skill : skills) {
                if (skill.metadata().name() != null) {
                    prompt.append("- **").append(skill.metadata().name()).append("**");
                    if (skill.metadata().description() != null) {
                        prompt.append(": ").append(skill.metadata().description());
                    }
                    prompt.append("\n");
                }
            }
            prompt.append("\n").append(SKILL_USAGE_HINT);
        }

        if (customPrompt != null && !customPrompt.isEmpty()) {
            prompt.append("\n\n## Custom Instructions\n\n").append(customPrompt);
        }

        return prompt.toString();
    }
}
