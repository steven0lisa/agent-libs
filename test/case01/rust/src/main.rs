use std::env;
use std::path::PathBuf;

use agentlib::{Agent, AgentConfig};

#[tokio::main]
async fn main() {
    let case_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let prompt_path = case_dir.join("prompt.txt");
    let prompt = tokio::fs::read_to_string(&prompt_path).await.unwrap();

    println!("=== Prompt ===");
    println!("{}", prompt);
    println!();

    let data_path = case_dir.join("data.txt");
    let data = tokio::fs::read_to_string(&data_path).await.unwrap();
    println!("=== Data Preview ===");
    for (i, line) in data.lines().enumerate() {
        if i >= 25 {
            println!("...");
            break;
        }
        println!("{}", line);
    }
    println!();

    let api_key = env::var("ANTHROPIC_AUTH_TOKEN")
        .or_else(|_| env::var("ANTHROPIC_API_KEY"))
        .unwrap_or_default();
    if api_key.is_empty() {
        println!("ERROR: ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY not set");
        return;
    }

    let config = AgentConfig {
        api_key,
        base_url: env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
        model: env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".to_string()),
        work_dir: case_dir.clone(),
        max_turns: 10,
        ..Default::default()
    };

    let mut agent = Agent::new(config);
    println!("=== Agent Output ===\n");

    let mut rx = agent.run(&prompt).await.unwrap();

    while let Some(event) = rx.recv().await {
        match event {
            agentlib::Event::MessageDelta { text } => print!("{}", text),
            agentlib::Event::ToolUseStart { name, input, .. } => {
                println!("\n[Tool call: {}] input={}", name, input);
            }
            agentlib::Event::Complete { final_content } => {
                println!("\n\n=== Complete ===");
                println!("{}", final_content);
            }
            agentlib::Event::Error { message } => {
                println!("\n[Error] {}", message);
            }
            _ => {}
        }
    }

    println!("\n[Final state] {:?}", agent.state().await);
}
