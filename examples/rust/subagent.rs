// Example: Enable subagent support.
use agentlib::{Agent, AgentConfig};
use std::env;

#[tokio::main]
async fn main() {
    let config = AgentConfig {
        api_key: env::var("ANTHROPIC_AUTH_TOKEN").unwrap_or_default(),
        enable_subagent: true,
        subagent_max_turns: 20,
        ..Default::default()
    };

    let mut agent = Agent::new(config);
    let mut rx = agent.run(
        "Explore this codebase using subagents to parallelize exploration"
    ).await.unwrap();

    while let Some(event) = rx.recv().await {
        match event {
            agentlib::Event::MessageDelta { text } => print!("{}", text),
            agentlib::Event::Complete { final_content } => println!("\n\n[Complete] {}", final_content),
            agentlib::Event::Error { message } => println!("\n[Error] {}", message),
            _ => {}
        }
    }
}
