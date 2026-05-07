// Basic usage example: Run the agent with a simple prompt.
use agentlib::{Agent, AgentConfig};
use std::env;

#[tokio::main]
async fn main() {
    let config = AgentConfig {
        api_key: env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| env::var("ANTHROPIC_API_KEY"))
            .unwrap_or_default(),
        base_url: env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| "https://api.anthropic.com".into()),
        model: env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".into()),
        ..Default::default()
    };

    let mut agent = Agent::new(config);
    let mut rx = agent.run("Please list the files in the current directory").await.unwrap();

    while let Some(event) = rx.recv().await {
        match event {
            agentlib::Event::MessageDelta { text } => print!("{}", text),
            agentlib::Event::Complete { final_content } => {
                println!("\n\n[Complete] {}", final_content);
            }
            agentlib::Event::Error { message } => {
                println!("\n[Error] {}", message);
            }
            _ => {}
        }
    }
}
