// Example: Use a callback to log every step.
use agentlib::{Agent, AgentConfig, Event};
use std::env;

fn on_event(event: Event) {
    match event {
        Event::ToolUseStart { name, .. } => println!("[Callback] Tool '{}' called", name),
        Event::ToolUseEnd { name, result } => {
            let status = if result.is_error { "ERROR" } else { "OK" };
            println!("[Callback] Tool '{}' finished: {}", name, status);
        }
        Event::Error { message } => println!("[Callback] Error: {}", message),
        _ => {}
    }
}

#[tokio::main]
async fn main() {
    let config = AgentConfig {
        api_key: env::var("ANTHROPIC_AUTH_TOKEN").unwrap_or_default(),
        callback: Some(Box::new(on_event)),
        ..Default::default()
    };

    let mut agent = Agent::new(config);
    let mut rx = agent.run("Read README.md and summarize it").await.unwrap();

    while let Some(event) = rx.recv().await {
        match event {
            agentlib::Event::MessageDelta { text } => print!("{}", text),
            agentlib::Event::Complete { final_content } => println!("\n\n[Complete] {}", final_content),
            _ => {}
        }
    }
}
