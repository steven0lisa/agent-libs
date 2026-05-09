package com.agentlib.skills;

import com.agentlib.Message;
import com.agentlib.Tool;
import com.agentlib.ToolContext;
import com.agentlib.ToolResult;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

/**
 * Tool that loads a skill and returns its instructions via message injection.
 * Skills provide specialized capabilities for specific tasks.
 */
public class SkillTool implements Tool {

    private final SkillLoader loader;

    public SkillTool(SkillLoader loader) {
        this.loader = loader;
    }

    @Override
    public String name() {
        return "skill";
    }

    @Override
    public String description() {
        String base = "Load a skill and get its instructions. " +
               "Skills provide specialized capabilities for specific tasks.";
        List<SkillInfo> all = loader.discoverAll();
        List<SkillInfo> invocable = all.stream()
            .filter(s -> s.metadata().userInvocable())
            .toList();
        if (invocable.isEmpty()) {
            return base;
        }
        StringBuilder sb = new StringBuilder(base);
        sb.append("\n\nAvailable skills:\n");
        for (SkillInfo skill : invocable) {
            if (skill.metadata().description() != null && !skill.metadata().description().isEmpty()) {
                sb.append(String.format("- %s: %s%n", skill.metadata().name(), skill.metadata().description()));
            } else {
                sb.append(String.format("- %s%n", skill.metadata().name()));
            }
        }
        return sb.toString();
    }

    @Override
    public boolean isReadOnly() {
        return true;
    }

    @Override
    public JsonNode inputSchema() {
        ObjectNode schema = JsonNodeFactory.instance.objectNode();
        schema.put("type", "object");

        ObjectNode properties = JsonNodeFactory.instance.objectNode();

        ObjectNode skill = JsonNodeFactory.instance.objectNode();
        skill.put("type", "string");
        skill.put("description", "The name of the skill to load");
        properties.set("skill", skill);

        ObjectNode args = JsonNodeFactory.instance.objectNode();
        args.put("type", "string");
        args.put("description",
            "Optional arguments passed to the skill via $ARGUMENTS variable substitution");
        properties.set("args", args);

        schema.set("properties", properties);
        schema.set("required", JsonNodeFactory.instance.arrayNode().add("skill"));
        return schema;
    }

    @Override
    public ToolResult call(Map<String, Object> input, ToolContext context) {
        String skillName = (String) input.get("skill");
        String args = (String) input.get("args");

        if (skillName == null || skillName.isEmpty()) {
            return ToolResult.error("\"skill\" is required");
        }

        var skillOpt = loader.findByName(skillName);
        if (skillOpt.isEmpty()) {
            List<SkillInfo> all = loader.discoverAll();
            String available = all.stream()
                .map(s -> s.metadata().name())
                .filter(n -> n != null)
                .collect(Collectors.joining(", "));
            String msg = "Skill not found: \"" + skillName + "\". Available skills: "
                + (available.isEmpty() ? "(none)" : available);
            return ToolResult.error(msg);
        }

        SkillInfo skill = skillOpt.get();

        String processedContent = VariableSubstitutor.substitute(
            skill.content(), args, skill.dirPath());

        String fullContent = buildSkillInjectionContent(skill, processedContent);
        String brief = "Skill loaded: " + skill.metadata().name();
        Message injectionMsg = Message.user(fullContent);
        return ToolResult.successWithMessages(brief, List.of(injectionMsg));
    }

    private static String buildSkillInjectionContent(SkillInfo skill, String content) {
        StringBuilder sb = new StringBuilder();
        sb.append("## Skill: ").append(skill.metadata().name()).append("\n");

        if (skill.metadata().description() != null && !skill.metadata().description().isEmpty()) {
            sb.append("Description: ").append(skill.metadata().description()).append("\n");
        }
        if (skill.metadata().whenToUse() != null && !skill.metadata().whenToUse().isEmpty()) {
            sb.append("When to use: ").append(skill.metadata().whenToUse()).append("\n");
        }

        sb.append("\n").append(content);

        if (skill.metadata().allowedTools() != null && !skill.metadata().allowedTools().isEmpty()) {
            sb.append("\n\nNote: When following this skill's instructions, only use these tools: ")
              .append(String.join(", ", skill.metadata().allowedTools()));
        }

        if ("fork".equals(skill.metadata().context())) {
            sb.append("\n\nThis skill should be executed in a fork context.");
        }

        return sb.toString();
    }
}
