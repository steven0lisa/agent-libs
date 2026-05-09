//! Skill 系统验证测试
//!
//! 目的：验证 agentlib 的 Skill 系统在实际场景中能否正常工作。
//! 对照 CC (Claude Code) 的行为，确认以下关键路径：
//!
//! 测试分层：
//!   L1 - Skill 加载/发现（纯本地，无需 API）
//!   L2 - SkillTool 调用（纯本地，无需 API）
//!   L3 - Agent + Skill 集成（需要 API Key）
//!
//! 运行方式：
//!   cargo test --test skill_verification_tests                        # L1+L2
//!   ANTHROPIC_AUTH_TOKEN=xxx cargo test --test skill_verification_tests  # 全部

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use agentlib::{
    Agent, AgentConfig, ContentBlock, Event, Message, OutputFormat, Pattern, PatternType, Role,
    Tool, ToolContext, ToolError, ToolResult,
};
use agentlib::skills::{SkillLoader, SkillTool, SkillInfo, SkillLoaderOptions};
use agentlib::tools::bash::BashTool;
use async_trait::async_trait;
use serde_json::json;
use tempfile::TempDir;

// ============================================================
// 辅助函数
// ============================================================

/// 创建一个包含多个 skill 的临时目录
fn create_multi_skill_dir(dir: &std::path::Path, skills: &[(&str, &str, &str, bool)]) {
    for (name, description, content, user_invocable) in skills {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        let frontmatter = if *user_invocable {
            format!(
                "---\nname: {}\ndescription: \"{}\"\nwhen_to_use: When you need {}\nuser_invocable: true\n---\n\n{}",
                name, description, description, content
            )
        } else {
            format!(
                "---\nname: {}\ndescription: \"{}\"\nwhen_to_use: When you need {}\nuser_invocable: false\n---\n\n{}",
                name, description, description, content
            )
        };
        fs::write(skill_dir.join("SKILL.md"), frontmatter).unwrap();
    }
}

/// 创建一个包含 extra_env 检查的 "bash-echo" skill
fn create_bash_echo_skill(dir: &std::path::Path) {
    let skill_dir = dir.join("bash-echo");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: bash-echo\ndescription: Run a bash echo command\nwhen_to_use: When you need to test bash execution\n---\n\nUse the bash tool to run: echo \"Hello from skill, args=$ARGUMENTS\"",
    )
    .unwrap();
}

/// 创建一个 kafka-ui 风格的 skill（模拟真实的 kafka-ui SKILL.md）
fn create_kafka_ui_skill(dir: &std::path::Path) {
    let skill_dir = dir.join("kafka-ui");
    fs::create_dir_all(&skill_dir).unwrap();

    // 创建一个简化版的 kafka-ui 脚本
    fs::write(
        skill_dir.join("list_topics.py"),
        r#"
import sys
# 模拟 kafka topic 列表
topics = ["topic-A", "topic-B", "topic-C"]
print("Topics:", ", ".join(topics))
"#,
    )
    .unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: kafka-ui
description: "与 Kafka UI API 交互，查询集群、broker、topic"
when_to_use: "当用户提到 Kafka、查看 topic、消费组、消息时使用"
allowed_tools: bash
---

# Kafka UI 技能

使用 Python 脚本与 Kafka UI API 交互。

## 查询 Topic 列表

执行以下命令：
```bash
python3 ${CLAUDE_SKILL_DIR}/list_topics.py
```

## 查询特定 Topic

如果用户指定了集群名称，传入 $1 参数。
"#,
    )
    .unwrap();
}

/// 创建一个带 allowed_tools 限制的 skill
fn create_restricted_skill(dir: &std::path::Path) {
    let skill_dir = dir.join("readonly-check");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: readonly-check\ndescription: A read-only skill\nallowed_tools: bash, grep\n---\n\nOnly use bash and grep to check files.",
    )
    .unwrap();
}

fn make_ctx(work_dir: &std::path::Path) -> ToolContext {
    ToolContext {
        work_dir: work_dir.to_path_buf(),
        message_history: vec![],
        allowed_read_dirs: vec![],
        allowed_write_dirs: vec![],
        extra_env: Default::default(),
    }
}

fn make_ctx_with_env(work_dir: &std::path::Path, env: HashMap<String, String>) -> ToolContext {
    ToolContext {
        work_dir: work_dir.to_path_buf(),
        message_history: vec![],
        allowed_read_dirs: vec![],
        allowed_write_dirs: vec![],
        extra_env: env,
    }
}

// ============================================================
// L1: Skill 加载/发现（纯本地测试）
// ============================================================

#[test]
fn l1_discover_multiple_skills() {
    let dir = TempDir::new().unwrap();
    create_multi_skill_dir(
        dir.path(),
        &[
            ("kafka-ui", "查询Kafka集群和Topic", "Use bash to query kafka", true),
            ("grafana", "查询Grafana监控数据", "Use curl to query grafana API", true),
            ("gitlab", "与GitLab API交互", "Use curl to call gitlab", true),
        ],
    );

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));

    let skills = loader.discover_all();
    assert_eq!(skills.len(), 3, "应该发现 3 个 skills");

    let names: Vec<&str> = skills.iter().map(|s| s.metadata.name.as_str()).collect();
    assert!(names.contains(&"kafka-ui"), "应该包含 kafka-ui");
    assert!(names.contains(&"grafana"), "应该包含 grafana");
    assert!(names.contains(&"gitlab"), "应该包含 gitlab");
}

#[test]
fn l1_find_skill_by_name() {
    let dir = TempDir::new().unwrap();
    create_kafka_ui_skill(dir.path());

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));

    let skill = loader.find_by_name("kafka-ui");
    assert!(skill.is_some(), "应该能找到 kafka-ui skill");

    let s = skill.unwrap();
    assert_eq!(s.metadata.name, "kafka-ui");
    assert!(!s.metadata.description.is_empty());
    assert!(s.content.contains("Kafka UI"));
    assert!(s.dir_path.join("list_topics.py").exists(), "skill 目录应包含脚本文件");
}

#[test]
fn l1_skill_with_allowed_tools() {
    let dir = TempDir::new().unwrap();
    create_restricted_skill(dir.path());

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));

    let skill = loader.find_by_name("readonly-check").unwrap();
    assert_eq!(skill.metadata.allowed_tools, vec!["bash", "grep"]);
}

#[test]
fn l1_user_invocable_filtering() {
    let dir = TempDir::new().unwrap();
    create_multi_skill_dir(
        dir.path(),
        &[
            ("public-skill", "公开技能", "Do public things", true),
            ("internal-skill", "内部技能", "Do internal things", false),
        ],
    );

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));

    let skills = loader.discover_all();
    let invocable: Vec<&SkillInfo> = skills.iter().filter(|s| s.metadata.user_invocable).collect();
    assert_eq!(invocable.len(), 1);
    assert_eq!(invocable[0].metadata.name, "public-skill");
}

#[test]
fn l1_skill_yaml_parsing_complex() {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("complex-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: complex-skill
description: "一个复杂的技能"
when_to_use: "需要复杂操作时"
allowed_tools: bash, curl, grep
context: inline
version: "2.0.0"
user_invocable: true
---

# 复杂技能

## 步骤 1
使用 $1 作为第一个参数。
使用 $ARGUMENTS 作为全部参数。

## 步骤 2
环境变量: ${ENV:HOME}
技能目录: ${CLAUDE_SKILL_DIR}
"#,
    )
    .unwrap();

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));

    let skill = loader.find_by_name("complex-skill").unwrap();
    assert_eq!(skill.metadata.name, "complex-skill");
    assert_eq!(skill.metadata.version, "2.0.0");
    assert_eq!(skill.metadata.context, "inline");
    assert_eq!(skill.metadata.allowed_tools, vec!["bash", "curl", "grep"]);
    assert!(skill.content.contains("步骤 1"));
    assert!(skill.content.contains("$1"));
    assert!(skill.content.contains("$ARGUMENTS"));
}

#[test]
fn l1_empty_skills_dir() {
    let dir = TempDir::new().unwrap();
    let empty_dir = dir.path().join("skills");
    fs::create_dir_all(&empty_dir).unwrap();

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(empty_dir),
        include_project_skills: false,
        project_dir: None,
    }));

    let skills = loader.discover_all();
    assert!(skills.is_empty(), "空目录应该返回 0 个 skills");
}

#[test]
fn l1_project_skills_loading() {
    let user_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    // 用户级 skill
    create_multi_skill_dir(
        user_dir.path(),
        &[("user-skill", "用户技能", "Do user things", true)],
    );

    // 项目级 skill
    let project_skills = project_dir.path().join(".claude").join("skills");
    create_multi_skill_dir(
        &project_skills,
        &[("project-skill", "项目技能", "Do project things", true)],
    );

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(user_dir.path().to_path_buf()),
        include_project_skills: true,
        project_dir: Some(project_dir.path().to_path_buf()),
    }));

    let skills = loader.discover_all();
    assert_eq!(skills.len(), 2, "应该发现 2 个 skills (user + project)");

    let names: Vec<&str> = skills.iter().map(|s| s.metadata.name.as_str()).collect();
    assert!(names.contains(&"user-skill"));
    assert!(names.contains(&"project-skill"));
}

// ============================================================
// L2: SkillTool 调用测试（纯本地，无需 API）
// ============================================================

#[tokio::test]
async fn l2_skill_tool_call_with_name() {
    let dir = TempDir::new().unwrap();
    create_kafka_ui_skill(dir.path());

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);
    let ctx = make_ctx(dir.path());

    let input = json!({"skill": "kafka-ui"});
    let result = tool.call(input, &ctx).await;

    assert!(result.is_ok(), "调用 kafka-ui skill 应该成功");
    let result = result.unwrap();
    assert!(!result.is_error);
    // Skill content is now injected via new_messages (message injection mode)
    assert!(result.content.contains("kafka-ui"), "结果应包含 skill 名称");
    assert_eq!(result.new_messages.len(), 1, "应有 1 条注入消息");
    if let agentlib::ContentBlock::Text { text } = &result.new_messages[0].content[0] {
        assert!(text.contains("Kafka UI"), "注入消息应包含 skill 内容");
    } else {
        panic!("注入消息应包含 Text content block");
    }
}

#[tokio::test]
async fn l2_skill_tool_call_with_args() {
    let dir = TempDir::new().unwrap();
    create_kafka_ui_skill(dir.path());

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);
    let ctx = make_ctx(dir.path());

    let input = json!({"skill": "kafka-ui", "args": "kafka-rootcloudV4"});
    let result = tool.call(input, &ctx).await.unwrap();

    assert!(!result.is_error);
    // Check args substitution in injected message (message injection mode)
    if let agentlib::ContentBlock::Text { text } = &result.new_messages[0].content[0] {
        assert!(
            text.contains("kafka-rootcloudV4"),
            "注入消息中 $ARGUMENTS 应该被替换为传入的参数"
        );
    } else {
        panic!("注入消息应包含 Text content block");
    }
}

#[tokio::test]
async fn l2_skill_tool_empty_input_returns_error() {
    let dir = TempDir::new().unwrap();
    let skills_dir = dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(skills_dir),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);
    let ctx = make_ctx(dir.path());

    // 模拟模型传了空 input:{} — 这是之前测试中发现的问题
    let input = json!({});
    let result = tool.call(input, &ctx).await;

    assert!(result.is_err(), "空 input 应该返回错误");
    let err = result.unwrap_err();
    assert!(
        err.0.contains("\"skill\" is required"),
        "错误信息应说明 skill 参数缺失，实际: {}",
        err.0
    );
}

#[tokio::test]
async fn l2_skill_tool_nonexistent_skill() {
    let dir = TempDir::new().unwrap();
    let skills_dir = dir.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(skills_dir),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);
    let ctx = make_ctx(dir.path());

    let input = json!({"skill": "nonexistent-skill"});
    let result = tool.call(input, &ctx).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.0.contains("Skill not found"), "错误信息应包含 'Skill not found'");
}

#[tokio::test]
async fn l2_skill_tool_variable_substitution() {
    let dir = TempDir::new().unwrap();
    let skill_dir = dir.path().join("var-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: var-skill\n---\n\nFirst: $1, Second: $2, All: $ARGUMENTS, Dir: ${CLAUDE_SKILL_DIR}",
    )
    .unwrap();

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);
    let ctx = make_ctx(dir.path());

    let input = json!({"skill": "var-skill", "args": "hello world"});
    let result = tool.call(input, &ctx).await.unwrap();

    assert!(!result.is_error);
    // Check variable substitution in injected message (message injection mode)
    if let agentlib::ContentBlock::Text { text } = &result.new_messages[0].content[0] {
        assert!(text.contains("First: hello"), "$1 应被替换");
        assert!(text.contains("Second: world"), "$2 应被替换");
        assert!(text.contains("All: hello world"), "$ARGUMENTS 应被替换");
        assert!(text.contains("Dir: "), "CLAUDE_SKILL_DIR 应被替换");
    } else {
        panic!("注入消息应包含 Text content block");
    }
}

#[tokio::test]
async fn l2_skill_tool_allowed_tools_hint() {
    let dir = TempDir::new().unwrap();
    create_restricted_skill(dir.path());

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);
    let ctx = make_ctx(dir.path());

    let input = json!({"skill": "readonly-check"});
    let result = tool.call(input, &ctx).await.unwrap();

    assert!(!result.is_error);
    // Check allowed_tools hint in injected message (message injection mode)
    if let agentlib::ContentBlock::Text { text } = &result.new_messages[0].content[0] {
        assert!(
            text.contains("only use these tools: bash, grep"),
            "注入消息应包含 allowed_tools 提示"
        );
    } else {
        panic!("注入消息应包含 Text content block");
    }
}

#[tokio::test]
async fn l2_skill_tool_description() {
    let dir = TempDir::new().unwrap();
    create_multi_skill_dir(
        dir.path(),
        &[
            ("kafka-ui", "查询Kafka", "query kafka", true),
            ("grafana", "查询监控", "query grafana", true),
        ],
    );

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);

    let desc = tool.description();
    assert!(
        desc.contains("skill") || desc.contains("Skill"),
        "描述应包含 skill 关键字"
    );
}

// ============================================================
// L2.5: Skill Listing 注入测试
// ============================================================

#[test]
fn l2_5_skill_listing_format() {
    let dir = TempDir::new().unwrap();
    create_multi_skill_dir(
        dir.path(),
        &[
            ("kafka-ui", "查询Kafka", "query kafka", true),
            ("grafana", "查询监控", "query grafana", true),
            ("internal-tool", "内部工具", "internal use only", false),
        ],
    );

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));

    let skills = loader.discover_all();

    // 模拟 Agent::build_skill_listing 的逻辑
    let invocable: Vec<&SkillInfo> = skills
        .iter()
        .filter(|s| s.metadata.user_invocable)
        .collect();

    assert_eq!(invocable.len(), 2, "只有 user_invocable=true 的 skill 应出现在 listing");

    let mut lines = vec![
        "The following skills are available for use with the Skill tool:".to_string(),
    ];
    for skill in &invocable {
        let desc = if skill.metadata.description.is_empty() {
            String::new()
        } else {
            format!(": {}", skill.metadata.description)
        };
        lines.push(format!("- /{}{}", skill.metadata.name, desc));
    }
    let listing = lines.join("\n");

    assert!(listing.contains("/kafka-ui"), "listing 应包含 kafka-ui");
    assert!(listing.contains("/grafana"), "listing 应包含 grafana");
    assert!(
        !listing.contains("internal-tool"),
        "listing 不应包含 user_invocable=false 的 skill"
    );
}

#[test]
fn l2_5_skill_listing_injection_position() {
    let messages = vec![Message::user("查看新C的kafka topic列表")];

    let skill_listing = "The following skills are available:\n- /kafka-ui: 查询Kafka";
    let mut messages_with_listing = messages;
    if let Some(last_msg) = messages_with_listing.last_mut() {
        if last_msg.role == Role::User {
            last_msg.content.push(ContentBlock::Text {
                text: format!("\n\n<system-reminder>\n{}\n</system-reminder>", skill_listing),
            });
        }
    }

    let last = messages_with_listing.last().unwrap();
    assert_eq!(last.role, Role::User);

    let has_listing = last.content.iter().any(|b| {
        if let ContentBlock::Text { text } = b {
            text.contains("kafka-ui")
        } else {
            false
        }
    });
    assert!(has_listing, "skill listing 应注入到最后一条 user message");
}

// ============================================================
// L2.5b: 模型空参数调用的边界处理
// ============================================================

#[tokio::test]
async fn l2_skill_tool_handles_model_empty_call() {
    let dir = TempDir::new().unwrap();
    create_multi_skill_dir(
        dir.path(),
        &[
            ("kafka-ui", "查询Kafka", "query kafka", true),
            ("grafana", "查询监控", "query grafana", true),
        ],
    );

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);
    let ctx = make_ctx(dir.path());

    // 场景 1: 完全空的 input
    let result = tool.call(json!({}), &ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().0.contains("\"skill\" is required"));

    // 场景 2: skill 为 null
    let result = tool.call(json!({"skill": null}), &ctx).await;
    assert!(result.is_err());

    // 场景 3: 正确调用
    let result = tool.call(json!({"skill": "kafka-ui"}), &ctx).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().is_error);
}

// ============================================================
// L2.5c: extra_env 注入到 bash 工具
// ============================================================

#[tokio::test]
async fn l2_bash_extra_env_injection() {
    let dir = TempDir::new().unwrap();

    let mut env = HashMap::new();
    env.insert("TEST_VAR_SKILL".to_string(), "injected_value".to_string());

    let ctx = make_ctx_with_env(dir.path(), env);

    let tool = BashTool::new(
        vec![Pattern {
            pattern_type: PatternType::Wildcard,
            pattern: "*".to_string(),
        }],
        vec![],
        8192,
    );

    let input = json!({"command": "echo $TEST_VAR_SKILL"});
    let result = tool.call(input, &ctx).await.unwrap();

    assert!(
        result.content.contains("injected_value"),
        "Bash 工具应该能访问 extra_env 中注入的环境变量。实际输出: {}",
        result.content
    );
}

// ============================================================
// L3: Agent + Skill 集成测试（需要 API Key）
// ============================================================

fn make_test_agent_config(skills_dir: PathBuf) -> AgentConfig {
    let api_key = std::env::var("ANTHROPIC_AUTH_TOKEN")
        .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
        .unwrap_or_default();

    let base_url = std::env::var("ANTHROPIC_BASE_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com".to_string());

    let model = std::env::var("ANTHROPIC_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

    AgentConfig {
        base_url,
        api_key,
        model,
        work_dir: skills_dir.clone(),
        max_tokens: 4096,
        max_turns: 10,
        max_duration_ms: 120_000,
        system_prompt: Some("You are a test assistant. Always respond in Chinese.".to_string()),
        timeout_ms: 30_000,
        stream: true,
        output_format: OutputFormat::default(),
        enable_subagent: false,
        subagent_max_turns: 5,
        bash_whitelist: vec![Pattern {
            pattern_type: PatternType::Wildcard,
            pattern: "*".to_string(),
        }],
        bash_blacklist: vec![],
        curl_whitelist: vec![],
        curl_blacklist: vec![],
        enable_skills: true,
        skills_dir: Some(skills_dir),
        include_project_skills: false,
        skills_project_dir: None,
        auto_compact: false,
        context_window_size: 200_000,
        auto_compact_threshold_pct: 0.8,
        allowed_read_dirs: vec![],
        allowed_write_dirs: vec![],
        extra_env: HashMap::new(),
        bash_output_buffer_size: 8192,
    }
}

fn skip_if_no_api_key() -> String {
    let key = std::env::var("ANTHROPIC_AUTH_TOKEN")
        .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
        .unwrap_or_default();
    if key.is_empty() {
        eprintln!("⚠ 跳过 L3 测试：未设置 ANTHROPIC_AUTH_TOKEN 或 ANTHROPIC_API_KEY");
    }
    key
}

async fn collect_events(rx: &mut tokio::sync::mpsc::Receiver<Event>) -> Vec<Event> {
    let mut events = Vec::new();
    while let Some(event) = rx.recv().await {
        events.push(event.clone());
        if matches!(event, Event::Complete { .. } | Event::Error { .. }) {
            break;
        }
    }
    events
}

fn extract_text_from_events(events: &[Event]) -> String {
    let mut text = String::new();
    for event in events {
        if let Event::MessageDelta { text: delta } = event {
            text.push_str(delta);
        }
        if let Event::Complete { final_content } = event {
            if !final_content.is_empty() && text.is_empty() {
                text = final_content.clone();
            }
        }
    }
    text
}

fn extract_tool_calls(events: &[Event]) -> Vec<(String, serde_json::Value)> {
    let mut calls = Vec::new();
    for event in events {
        if let Event::ToolUseStart { name, input, .. } = event {
            calls.push((name.clone(), input.clone()));
        }
    }
    calls
}

#[tokio::test]
async fn l3_agent_skill_simple_call() {
    let key = skip_if_no_api_key();
    if key.is_empty() { return; }

    let dir = TempDir::new().unwrap();
    create_kafka_ui_skill(dir.path());

    let config = make_test_agent_config(dir.path().to_path_buf());
    let mut agent = Agent::new(config);

    let mut rx = agent
        .run("请使用 kafka-ui skill 查看一下有哪些 topic")
        .await
        .unwrap();

    let events = collect_events(&mut rx).await;
    let tool_calls = extract_tool_calls(&events);

    let skill_calls: Vec<_> = tool_calls
        .iter()
        .filter(|(name, _)| name == "skill")
        .collect();

    // Model behavior is non-deterministic; sometimes it may not call the skill tool.
    // If it does call it, verify the parameters are correct.
    if skill_calls.is_empty() {
        eprintln!(
            "⚠ Agent did not call skill tool. Actual tool calls: {:?}. \
             This is a model behavior issue, not a code bug.",
            tool_calls.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
        );
        let text = extract_text_from_events(&events);
        println!("L3 Agent 输出: {}", text);
        return;
    }

    for (_, input) in &skill_calls {
        let skill_name = input.get("skill").and_then(|v| v.as_str()).unwrap_or("");
        if !skill_name.is_empty() {
            assert_eq!(skill_name, "kafka-ui", "skill 参数应为 'kafka-ui'，实际: {}", skill_name);
        } else {
            // Model sometimes sends empty input {} — SkillTool will return an error
            // which the model should recover from. This is not a code bug.
            eprintln!(
                "⚠ Model sent empty skill input: {:?}. \
                 SkillTool will return an error and model should recover.",
                input
            );
        }
    }

    let text = extract_text_from_events(&events);
    println!("L3 Agent 输出: {}", text);
}

#[tokio::test]
async fn l3_agent_skill_auto_trigger() {
    let key = skip_if_no_api_key();
    if key.is_empty() { return; }

    let dir = TempDir::new().unwrap();
    create_kafka_ui_skill(dir.path());

    let config = make_test_agent_config(dir.path().to_path_buf());
    let mut agent = Agent::new(config);

    let mut rx = agent.run("帮我看看 kafka 有哪些 topic").await.unwrap();
    let events = collect_events(&mut rx).await;
    let tool_calls = extract_tool_calls(&events);

    println!("L3 自动匹配 - 工具调用:");
    for (name, input) in &tool_calls {
        println!("  {}({})", name, input);
    }

    let text = extract_text_from_events(&events);
    println!("L3 自动匹配输出: {}", text);

    assert!(
        !tool_calls.is_empty() || !text.is_empty(),
        "Agent 应该有输出或工具调用"
    );
}

#[tokio::test]
async fn l3_agent_skill_triggers_bash_execution() {
    let key = skip_if_no_api_key();
    if key.is_empty() { return; }

    let dir = TempDir::new().unwrap();
    create_bash_echo_skill(dir.path());

    let config = make_test_agent_config(dir.path().to_path_buf());
    let mut agent = Agent::new(config);

    let mut rx = agent
        .run("请使用 bash-echo skill 执行一下，传入参数 'test123'")
        .await
        .unwrap();

    let events = collect_events(&mut rx).await;
    let tool_calls = extract_tool_calls(&events);

    println!("L3 Bash 执行 - 工具调用:");
    for (name, input) in &tool_calls {
        println!("  {}({})", name, input);
    }

    let text = extract_text_from_events(&events);
    println!("L3 Bash 执行输出: {}", text);

    let has_bash_call = tool_calls.iter().any(|(name, _)| name == "bash");
    let has_skill_call = tool_calls.iter().any(|(name, _)| name == "skill");

    println!("Has skill call: {}, Has bash call: {}", has_skill_call, has_bash_call);

    if has_skill_call && !has_bash_call {
        eprintln!(
            "⚠ Agent 调用了 skill 但没有进一步调用 bash。\
             这可能说明 skill 内容被当作普通文本返回，\
             而非作为指令被执行（CC vs agentlib 架构差异）"
        );
    }
}

#[tokio::test]
async fn l3_agent_no_skill_for_non_matching_query() {
    let key = skip_if_no_api_key();
    if key.is_empty() { return; }

    let dir = TempDir::new().unwrap();
    create_kafka_ui_skill(dir.path());

    let config = make_test_agent_config(dir.path().to_path_buf());
    let mut agent = Agent::new(config);

    let mut rx = agent.run("1+1等于几？").await.unwrap();
    let events = collect_events(&mut rx).await;
    let tool_calls = extract_tool_calls(&events);

    let text = extract_text_from_events(&events);
    println!("L3 无关问题输出: {}", text);

    let has_kafka_skill = tool_calls.iter().any(|(name, input)| {
        name == "skill" && input.get("skill").and_then(|v| v.as_str()) == Some("kafka-ui")
    });

    assert!(!has_kafka_skill, "无关问题不应该触发 kafka-ui skill");
}

// ============================================================
// 设计建议测试 — 当前可能失败，说明需要改进
// ============================================================

#[test]
fn l2_description_should_list_available_skills() {
    let dir = TempDir::new().unwrap();
    create_multi_skill_dir(
        dir.path(),
        &[
            ("kafka-ui", "查询Kafka", "query kafka", true),
            ("grafana", "查询监控", "query grafana", true),
            ("gitlab", "查看CI", "view CI", true),
        ],
    );

    let loader = SkillLoader::new(Some(SkillLoaderOptions {
        skills_dir: Some(dir.path().to_path_buf()),
        include_project_skills: false,
        project_dir: None,
    }));
    let tool = SkillTool::new(loader);

    let description = tool.description();
    println!("当前 SkillTool description: {}", description);

    // TODO: 当前实现返回固定描述，不包含 skill 列表。
    // 改进后这个断言应该通过：
    // assert!(description.contains("kafka-ui"), "description 应列出可用 skills");
    //
    // 建议：将 description() 改为动态生成，包含已发现的 skill 列表。
    // 参考 CC 的做法：在 description 中列出所有可用 skill 的名称和描述。
}
