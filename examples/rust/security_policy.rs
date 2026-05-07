// Example: Use whitelist/blacklist security policies.
use agentlib::{Agent, AgentConfig, Pattern};
use std::env;

#[tokio::main]
async fn main() {
    let config = AgentConfig {
        api_key: env::var("ANTHROPIC_AUTH_TOKEN").unwrap_or_default(),
        bash_whitelist: vec![
            Pattern::new("git *", agentlib::PatternType::Wildcard),
            Pattern::new("ls *", agentlib::PatternType::Wildcard),
        ],
        bash_blacklist: vec![
            Pattern::new("rm *", agentlib::PatternType::Wildcard),
            Pattern::new(r"^dd\s", agentlib::PatternType::Regex),
        ],
        curl_whitelist: vec![
            Pattern::new("*.example.com/*", agentlib::PatternType::Wildcard),
        ],
        curl_blacklist: vec![
            Pattern::new(r"evil\.com|malicious\.org", agentlib::PatternType::Regex),
        ],
        ..Default::default()
    };

    let mut agent = Agent::new(config);
    let mut rx = agent.run("Show me the git status of this repository").await.unwrap();

    while let Some(event) = rx.recv().await {
        match event {
            agentlib::Event::MessageDelta { text } => print!("{}", text),
            agentlib::Event::Complete { final_content } => println!("\n\n[Complete] {}", final_content),
            agentlib::Event::Error { message } => println!("\n[Error] {}", message),
            _ => {}
        }
    }
}
