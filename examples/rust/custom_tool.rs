// Example: Register a custom tool with the agent.
use agentlib::{Agent, AgentConfig, Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::env;

struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str { "calculator" }
    fn description(&self) -> &str { "Perform basic arithmetic." }
    fn is_read_only(&self) -> bool { true }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string", "enum": ["add", "subtract", "multiply", "divide"] },
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["operation", "a", "b"]
        })
    }

    async fn call(&self, input: serde_json::Value, _ctx: &ToolContext) -> Result<ToolResult, agentlib::ToolError> {
        let op = input["operation"].as_str().unwrap_or("");
        let a = input["a"].as_f64().unwrap_or(0.0);
        let b = input["b"].as_f64().unwrap_or(0.0);

        let result = match op {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Ok(ToolResult::error("Cannot divide by zero"));
                }
                a / b
            }
            _ => return Ok(ToolResult::error(&format!("Unknown operation: {}", op))),
        };

        Ok(ToolResult::success(&result.to_string()))
    }
}

#[tokio::main]
async fn main() {
    let config = AgentConfig {
        api_key: env::var("ANTHROPIC_AUTH_TOKEN").unwrap_or_default(),
        ..Default::default()
    };

    let mut agent = Agent::new(config);
    agent.register_tool(Box::new(CalculatorTool));

    let mut rx = agent.run("What is 123 multiplied by 456?").await.unwrap();
    while let Some(event) = rx.recv().await {
        match event {
            agentlib::Event::MessageDelta { text } => print!("{}", text),
            agentlib::Event::Complete { final_content } => println!("\n\n[Complete] {}", final_content),
            agentlib::Event::Error { message } => println!("\n[Error] {}", message),
            _ => {}
        }
    }
}
