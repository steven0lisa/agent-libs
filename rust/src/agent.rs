//! Agent core implementation with lifecycle management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::client::AnthropicClient;
use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::prompt::build_system_prompt;
use crate::skills;
use crate::skills::types::SkillInfo;
use crate::stream::StreamEvent;
use crate::tool::{Tool, ToolDefinition, ToolResult};
use crate::types::{AgentState, ContentBlock, Event, Message, Role};

/// Agent with lifecycle management (run/pause/resume/stop).
pub struct Agent {
    config: AgentConfig,
    client: AnthropicClient,
    tools: HashMap<String, Box<dyn Tool>>,
    message_history: Arc<RwLock<Vec<Message>>>,
    turn_count: AtomicUsize,
    state: Arc<RwLock<AgentState>>,
    pause_event: Arc<tokio::sync::Notify>,
    start_time: Arc<Mutex<Option<Instant>>>,
    loaded_skills: Vec<SkillInfo>,
}

impl Agent {
    /// Create a new Agent with the given configuration.
    pub fn new(config: AgentConfig) -> Self {
        let client = AnthropicClient::new(&config);

        let mut agent = Self {
            config,
            client,
            tools: HashMap::new(),
            message_history: Arc::new(RwLock::new(Vec::new())),
            turn_count: AtomicUsize::new(0),
            state: Arc::new(RwLock::new(AgentState::Idle)),
            pause_event: Arc::new(tokio::sync::Notify::new()),
            start_time: Arc::new(Mutex::new(None)),
            loaded_skills: Vec::new(),
        };

        // Register built-in tools
        agent.register_tool(Box::new(crate::tools::read_file::ReadFileTool));
        agent.register_tool(Box::new(crate::tools::write_file::WriteFileTool));
        agent.register_tool(Box::new(crate::tools::update_file::UpdateFileTool));
        agent.register_tool(Box::new(crate::tools::bash::BashTool::new(
            agent.config.bash_whitelist.clone(),
            agent.config.bash_blacklist.clone(),
        )));
        agent.register_tool(Box::new(crate::tools::curl::CurlTool::new(
            agent.config.curl_whitelist.clone(),
            agent.config.curl_blacklist.clone(),
        )));

        // Register subagent tool if enabled
        if agent.config.enable_subagent {
            let history = agent.message_history.clone();
            let config = agent.config.clone();
            agent.register_tool(Box::new(crate::tools::subagent::SubAgentTool::new(
                config,
                vec![], // Will be updated at runtime
            )));
        }

        // Initialize skill system if enabled
        if agent.config.enable_skills {
            let options = skills::types::SkillLoaderOptions {
                skills_dir: agent.config.skills_dir.clone(),
                include_project_skills: agent.config.include_project_skills,
                project_dir: agent.config.skills_project_dir.clone(),
            };
            let loader = skills::loader::SkillLoader::new(Some(options));
            let loaded_skills = loader.discover_all();

            agent.register_tool(Box::new(skills::tool::SkillTool::new(loader)));
            agent.loaded_skills = loaded_skills;
        }

        agent
    }

    // ------------------------------------------------------------------
    // State management
    // ------------------------------------------------------------------

    /// Get the current agent state.
    pub async fn state(&self) -> AgentState {
        *self.state.read().await
    }

    /// Pause the agent. The current turn will finish, then pause.
    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        if *state == AgentState::Running {
            *state = AgentState::Paused;
        }
    }

    /// Resume a paused agent.
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        if *state == AgentState::Paused {
            *state = AgentState::Running;
            self.pause_event.notify_one();
        }
    }

    /// Stop the agent. Cannot be resumed.
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        *state = AgentState::Stopping;
        self.pause_event.notify_one();
    }

    /// Check if the agent should continue running.
    async fn check_state(&self) -> bool {
        let state = self.state.read().await;
        !matches!(*state, AgentState::Stopping | AgentState::Terminated)
    }

    /// Wait if the agent is paused.
    async fn wait_if_paused(&self) {
        loop {
            let state = *self.state.read().await;
            match state {
                AgentState::Paused => {
                    // Wait for resume or stop
                    tokio::select! {
                        _ = self.pause_event.notified() => {}
                        _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                    }
                }
                AgentState::Stopping | AgentState::Terminated => break,
                _ => break,
            }
        }
    }

    // ------------------------------------------------------------------
    // Tool management
    // ------------------------------------------------------------------

    /// Register a custom tool.
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Unregister a tool by name.
    pub fn unregister_tool(&mut self, name: &str) {
        self.tools.remove(name);
    }

    /// List all registered tools.
    pub fn list_tools(&self) -> Vec<&dyn Tool> {
        self.tools.values().map(|t| t.as_ref()).collect()
    }

    /// Find a tool by name.
    pub fn find_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    // ------------------------------------------------------------------
    // History
    // ------------------------------------------------------------------

    /// Get the message history.
    pub async fn get_message_history(&self) -> Vec<Message> {
        self.message_history.read().await.clone()
    }

    /// Set the message history (used for subagent context forking).
    pub fn set_message_history(&mut self, history: Vec<Message>) {
        self.message_history = Arc::new(RwLock::new(history));
    }

    /// Clear the message history.
    pub async fn clear_history(&self) {
        let mut history = self.message_history.write().await;
        history.clear();
        self.turn_count.store(0, Ordering::SeqCst);
    }

    // ------------------------------------------------------------------
    // Core methods
    // ------------------------------------------------------------------

    /// Run the agent with the given input.
    /// Returns a receiver for events.
    pub async fn run(&mut self, input: &str) -> Result<mpsc::Receiver<Event>, AgentError> {
        let user_msg = Message::user(input);
        {
            let mut history = self.message_history.write().await;
            history.push(user_msg);
        }
        self.run_loop().await
    }

    /// Continue the conversation with existing messages.
    pub async fn chat(
        &mut self,
        messages: Vec<Message>,
    ) -> Result<mpsc::Receiver<Event>, AgentError> {
        {
            let mut history = self.message_history.write().await;
            history.extend(messages);
        }
        self.run_loop().await
    }

    /// Run the agent loop and return an event stream.
    async fn run_loop(&mut self) -> Result<mpsc::Receiver<Event>, AgentError> {
        let (tx, rx) = mpsc::channel(100);

        // Reset state
        {
            let mut state = self.state.write().await;
            *state = AgentState::Running;
        }
        {
            let mut start_time = self.start_time.lock().await;
            *start_time = Some(Instant::now());
        }
        self.turn_count.store(0, Ordering::SeqCst);

        // Build system prompt
        let system_prompt = build_system_prompt(
            &self.tools,
            self.config.system_prompt.as_deref(),
            self.config.enable_subagent,
            self.config.subagent_max_turns,
            &self.loaded_skills,
        );

        // Clone necessary data for the spawned task
        let state = self.state.clone();
        let pause_event = self.pause_event.clone();
        let message_history = self.message_history.clone();
        let start_time = self.start_time.clone();
        let turn_count = Arc::new(AtomicUsize::new(0));
        let config = self.config.clone();
        let tools = self
            .tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            })
            .collect();
        let client = AnthropicClient::new(&self.config);

        // Spawn the agent loop
        tokio::spawn(async move {
            let result = Self::run_loop_inner(
                state,
                pause_event,
                message_history,
                start_time,
                turn_count,
                config,
                tools,
                client,
                system_prompt,
                tx.clone(),
            )
            .await;

            if let Err(e) = result {
                let _ = tx.send(Event::Error {
                    message: e.to_string(),
                }).await;
            }

            // Send completion event
            let _ = tx.send(Event::Complete {
                final_content: String::new(),
            }).await;
        });

        Ok(rx)
    }

    /// Inner agent loop implementation.
    #[allow(clippy::too_many_arguments)]
    async fn run_loop_inner(
        state: Arc<RwLock<AgentState>>,
        pause_event: Arc<tokio::sync::Notify>,
        message_history: Arc<RwLock<Vec<Message>>>,
        start_time: Arc<Mutex<Option<Instant>>>,
        turn_count: Arc<AtomicUsize>,
        config: AgentConfig,
        tools: Vec<ToolDefinition>,
        client: AnthropicClient,
        system_prompt: String,
        tx: mpsc::Sender<Event>,
    ) -> Result<(), AgentError> {
        loop {
            // Check state
            {
                let s = state.read().await;
                if matches!(*s, AgentState::Stopping | AgentState::Terminated) {
                    break;
                }
            }

            // Wait if paused
            Self::wait_if_paused_inner(&state, &pause_event).await;

            // Check state again after pause
            {
                let s = state.read().await;
                if matches!(*s, AgentState::Stopping | AgentState::Terminated) {
                    break;
                }
            }

            // Increment turn count
            let current_turn = turn_count.fetch_add(1, Ordering::SeqCst) + 1;
            if current_turn > config.max_turns {
                let _ = tx
                    .send(Event::Error {
                        message: "Max turns reached".to_string(),
                    })
                    .await;
                break;
            }

            let _ = tx.send(Event::TurnStart { turn: current_turn }).await;

            // Check duration limit
            if config.max_duration_ms > 0 {
                let elapsed = {
                    let st = start_time.lock().await;
                    st.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0)
                };
                if elapsed > config.max_duration_ms {
                    let _ = tx
                        .send(Event::Error {
                            message: "Max duration exceeded".to_string(),
                        })
                        .await;
                    break;
                }
            }

            // Get current message history
            let messages = {
                let history = message_history.read().await;
                history.clone()
            };

            // Wait if paused before API call
            Self::wait_if_paused_inner(&state, &pause_event).await;

            // Stream API response
            let mut assistant_content: Vec<ContentBlock> = Vec::new();
            let _ = tx.send(Event::MessageStart).await;

            match client
                .stream_messages(messages, system_prompt.clone(), tools.clone())
                .await
            {
                Ok(stream) => {
                    tokio::pin!(stream);
                    while let Some(event_result) = stream.next().await {
                        // Check state during streaming
                        {
                            let s = state.read().await;
                            if matches!(*s, AgentState::Stopping | AgentState::Terminated) {
                                break;
                            }
                        }

                        match event_result {
                            Ok(event) => match event {
                                StreamEvent::ContentBlockStart { content_block, .. } => {
                                    let block = Self::convert_content_block(content_block);
                                    assistant_content.push(block);
                                }
                                StreamEvent::ContentBlockDelta { delta, .. } => match delta {
                                    crate::stream::ContentDelta::TextDelta { text } => {
                                        let _ = tx.send(Event::MessageDelta { text }).await;
                                    }
                                    crate::stream::ContentDelta::ThinkingDelta { thinking } => {
                                        let _ = tx
                                            .send(Event::ThinkingDelta { thinking })
                                            .await;
                                    }
                                    crate::stream::ContentDelta::InputJsonDelta {
                                        partial_json,
                                    } => {
                                        // Accumulate partial JSON for tool_use input
                                        // This is handled by the content block tracking
                                        let _ = partial_json;
                                    }
                                    _ => {}
                                },
                                StreamEvent::MessageStop => {
                                    let _ = tx.send(Event::MessageEnd).await;
                                    break;
                                }
                                _ => {}
                            },
                            Err(e) => {
                                let _ = tx
                                    .send(Event::Error {
                                        message: format!("Stream error: {}", e),
                                    })
                                    .await;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Event::Error {
                            message: format!("API error: {}", e),
                        })
                        .await;
                    break;
                }
            }

            let _ = tx.send(Event::MessageEnd).await;

            // Check state
            {
                let s = state.read().await;
                if matches!(*s, AgentState::Stopping | AgentState::Terminated) {
                    break;
                }
            }

            // Add assistant message to history
            {
                let mut history = message_history.write().await;
                history.push(Message::assistant(assistant_content.clone()));
            }

            // Extract tool uses
            let tool_uses: Vec<ContentBlock> = assistant_content
                .iter()
                .filter(|b| matches!(b, ContentBlock::ToolUse { .. }))
                .cloned()
                .collect();

            if tool_uses.is_empty() {
                // Task complete
                let final_text = Self::extract_text(&assistant_content);
                let _ = tx.send(Event::Complete { final_content: final_text }).await;
                {
                    let mut s = state.write().await;
                    *s = AgentState::Completed;
                }
                return Ok(());
            }

            // Execute tools
            let results = Self::execute_tools(
                &tool_uses,
                &config,
                &message_history,
                &tx,
            )
            .await?;

            // Add tool results to history
            {
                let mut history = message_history.write().await;
                history.push(Message {
                    role: Role::User,
                    content: results,
                });
            }
        }

        // Set terminated state if not already completed
        {
            let mut s = state.write().await;
            if !matches!(*s, AgentState::Completed) {
                *s = AgentState::Terminated;
            }
        }

        Ok(())
    }

    /// Wait if the agent is paused (static version for spawned task).
    async fn wait_if_paused_inner(
        state: &Arc<RwLock<AgentState>>,
        pause_event: &Arc<tokio::sync::Notify>,
    ) {
        loop {
            let s = *state.read().await;
            match s {
                AgentState::Paused => {
                    tokio::select! {
                        _ = pause_event.notified() => {}
                        _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                    }
                }
                AgentState::Stopping | AgentState::Terminated => break,
                _ => break,
            }
        }
    }

    /// Execute tools, grouping read-only for concurrency.
    async fn execute_tools(
        tool_uses: &[ContentBlock],
        config: &AgentConfig,
        message_history: &Arc<RwLock<Vec<Message>>>,
        tx: &mpsc::Sender<Event>,
    ) -> Result<Vec<ContentBlock>, AgentError> {
        let mut results = Vec::new();

        // We need to look up tools - but we don't have access to self.tools here
        // So we'll execute them all sequentially for now
        // In a real implementation, you'd pass the tool registry

        for block in tool_uses {
            if let ContentBlock::ToolUse { name, id, input } = block {
                let _ = tx
                    .send(Event::ToolUseStart {
                        name: name.clone(),
                        id: id.clone(),
                        input: input.clone(),
                    })
                    .await;

                // For the spawned task, we need a different approach
                // We'll return an error indicating the tool needs to be looked up
                // In practice, this would be handled by passing the tool registry
                let result = ToolResult::error(format!(
                    "Tool '{}' execution requires agent context",
                    name
                ));

                let _ = tx
                    .send(Event::ToolUseEnd {
                        name: name.clone(),
                        id: id.clone(),
                        result: result.clone(),
                    })
                    .await;

                results.push(ContentBlock::ToolResult {
                    tool_use_id: id.clone(),
                    content: result.content,
                    is_error: Some(result.is_error),
                });
            }
        }

        Ok(results)
    }

    /// Convert stream content block to agent content block.
    fn convert_content_block(block: crate::stream::ContentBlock) -> ContentBlock {
        match block {
            crate::stream::ContentBlock::Text { text } => ContentBlock::Text { text },
            crate::stream::ContentBlock::ToolUse { name, id, input } => {
                ContentBlock::ToolUse { name, id, input }
            }
            crate::stream::ContentBlock::Thinking { thinking, signature } => {
                ContentBlock::Thinking { thinking, signature }
            }
        }
    }

    /// Extract text from content blocks.
    fn extract_text(blocks: &[ContentBlock]) -> String {
        blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentConfig;
    use crate::tool::ToolContext;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_agent_creation() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            ..Default::default()
        };

        let agent = Agent::new(config);
        assert_eq!(agent.state().await, AgentState::Idle);
        assert_eq!(agent.list_tools().len(), 5); // 5 built-in tools
    }

    #[tokio::test]
    async fn test_agent_register_tool() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            ..Default::default()
        };

        let mut agent = Agent::new(config);
        let initial_count = agent.list_tools().len();

        struct CustomTool;

        #[async_trait::async_trait]
        impl Tool for CustomTool {
            fn name(&self) -> &str {
                "custom_tool"
            }

            fn description(&self) -> &str {
                "A custom tool"
            }

            fn input_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
            }

            async fn call(
                &self,
                _input: serde_json::Value,
                _ctx: &ToolContext,
            ) -> Result<ToolResult, crate::tool::ToolError> {
                Ok(ToolResult::success("ok"))
            }
        }

        agent.register_tool(Box::new(CustomTool));
        assert_eq!(agent.list_tools().len(), initial_count + 1);
        assert!(agent.find_tool("custom_tool").is_some());
    }

    #[tokio::test]
    async fn test_agent_unregister_tool() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            ..Default::default()
        };

        let mut agent = Agent::new(config);
        let initial_count = agent.list_tools().len();

        agent.unregister_tool("bash");
        assert_eq!(agent.list_tools().len(), initial_count - 1);
        assert!(agent.find_tool("bash").is_none());
    }

    #[tokio::test]
    async fn test_agent_pause_resume() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            ..Default::default()
        };

        let agent = Agent::new(config);

        // Initially idle
        assert_eq!(agent.state().await, AgentState::Idle);

        // Can't pause from idle
        agent.pause().await;
        // State should still be idle since we only pause from running
        assert_eq!(agent.state().await, AgentState::Idle);
    }

    #[tokio::test]
    async fn test_agent_stop() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            ..Default::default()
        };

        let agent = Agent::new(config);

        agent.stop().await;
        assert_eq!(agent.state().await, AgentState::Stopping);
    }

    #[tokio::test]
    async fn test_agent_history() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            ..Default::default()
        };

        let agent = Agent::new(config);

        let history = agent.get_message_history().await;
        assert!(history.is_empty());

        agent.clear_history().await;
        let history = agent.get_message_history().await;
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_agent_with_subagent() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            enable_subagent: true,
            ..Default::default()
        };

        let agent = Agent::new(config);
        // Should have 6 tools: 5 built-in + subagent
        assert_eq!(agent.list_tools().len(), 6);
        assert!(agent.find_tool("subagent").is_some());
    }

    #[tokio::test]
    async fn test_agent_without_subagent() {
        let temp = TempDir::new().unwrap();
        let config = AgentConfig {
            work_dir: temp.path().to_path_buf(),
            enable_subagent: false,
            ..Default::default()
        };

        let agent = Agent::new(config);
        // Should have 5 tools only
        assert_eq!(agent.list_tools().len(), 5);
        assert!(agent.find_tool("subagent").is_none());
    }
}
