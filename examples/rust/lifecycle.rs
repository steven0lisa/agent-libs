// Example: Manage agent lifecycle - pause, resume, and stop.
use agentlib::{Agent, AgentConfig};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let config = AgentConfig {
        api_key: env::var("ANTHROPIC_AUTH_TOKEN").unwrap_or_default(),
        ..Default::default()
    };

    let mut agent = Agent::new(config);

    // Schedule lifecycle changes.
    let agent_ref = &agent;
    tokio::spawn(async move {
        sleep(Duration::from_secs(3)).await;
        println!("\n[Lifecycle] Pausing agent...");
        agent_ref.pause();

        sleep(Duration::from_secs(2)).await;
        println!("\n[Lifecycle] Resuming agent...");
        agent_ref.resume();

        sleep(Duration::from_secs(5)).await;
        println!("\n[Lifecycle] Stopping agent...");
        agent_ref.stop();
    });

    let mut rx = agent.run("Explore the codebase and tell me about it").await.unwrap();
    while let Some(event) = rx.recv().await {
        match event {
            agentlib::Event::TurnStart { turn } => println!("\n[Turn {} started]", turn),
            agentlib::Event::MessageDelta { text } => print!("{}", text),
            agentlib::Event::Complete { final_content } => println!("\n\n[Complete] {}", final_content),
            agentlib::Event::Error { message } => println!("\n[Error] {}", message),
            _ => {}
        }
    }

    println!("\n[Final state] {:?}", agent.state());
}
