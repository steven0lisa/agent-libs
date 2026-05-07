package agentlib

import (
	"context"
	"sync"
	"testing"
	"time"
)

func TestNewAgent(t *testing.T) {
	cfg := DefaultConfig()
	agent := NewAgent(cfg)

	if agent == nil {
		t.Fatal("expected NewAgent to return non-nil agent")
	}
	if agent.State() != AgentStateIdle {
		t.Errorf("expected initial state to be idle, got %s", agent.State())
	}
	if agent.TurnCount() != 0 {
		t.Errorf("expected initial turn count to be 0, got %d", agent.TurnCount())
	}
	if len(agent.MessageHistory()) != 0 {
		t.Errorf("expected initial message history to be empty, got %d", len(agent.MessageHistory()))
	}
}

func TestAgentRegisterTool(t *testing.T) {
	agent := NewAgent(DefaultConfig())
	tool := &mockTool{name: "test_tool", description: "test"}

	agent.RegisterTool(tool)

	tools := agent.ListTools()
	if len(tools) != 1 {
		t.Fatalf("expected 1 tool, got %d", len(tools))
	}
	if tools[0].Name() != "test_tool" {
		t.Errorf("expected tool name 'test_tool', got %s", tools[0].Name())
	}

	got := agent.GetTool("test_tool")
	if got == nil {
		t.Error("expected GetTool to return non-nil")
	}
	if got.Name() != "test_tool" {
		t.Errorf("expected GetTool to return 'test_tool', got %s", got.Name())
	}
}

func TestAgentUnregisterTool(t *testing.T) {
	agent := NewAgent(DefaultConfig())
	tool := &mockTool{name: "test_tool", description: "test"}

	agent.RegisterTool(tool)
	agent.UnregisterTool("test_tool")

	if agent.GetTool("test_tool") != nil {
		t.Error("expected tool to be unregistered")
	}
	if len(agent.ListTools()) != 0 {
		t.Errorf("expected 0 tools, got %d", len(agent.ListTools()))
	}
}

func TestAgentClearHistory(t *testing.T) {
	agent := NewAgent(DefaultConfig())
	agent.messageHistory = append(agent.messageHistory, UserMessage("hello"))

	if len(agent.MessageHistory()) != 1 {
		t.Fatal("expected 1 message in history")
	}

	agent.ClearHistory()

	if len(agent.MessageHistory()) != 0 {
		t.Errorf("expected 0 messages after clear, got %d", len(agent.MessageHistory()))
	}
	if agent.TurnCount() != 0 {
		t.Errorf("expected turn count to be 0 after clear, got %d", agent.TurnCount())
	}
}

func TestAgentStateTransitions(t *testing.T) {
	agent := NewAgent(DefaultConfig())

	// Initial state
	if agent.State() != AgentStateIdle {
		t.Errorf("expected initial state idle, got %s", agent.State())
	}

	// Cannot pause idle agent
	agent.Pause()
	if agent.State() != AgentStateIdle {
		t.Errorf("expected state to remain idle after pause, got %s", agent.State())
	}

	// Simulate running state for pause test
	agent.mu.Lock()
	agent.state = AgentStateRunning
	agent.mu.Unlock()

	agent.Pause()
	if agent.State() != AgentStatePaused {
		t.Errorf("expected state paused, got %s", agent.State())
	}

	agent.Resume()
	if agent.State() != AgentStateRunning {
		t.Errorf("expected state running after resume, got %s", agent.State())
	}

	// Stop
	agent.Stop()
	if agent.State() != AgentStateStopping {
		t.Errorf("expected state stopping, got %s", agent.State())
	}
}

func TestAgentRunAlreadyRunning(t *testing.T) {
	agent := NewAgent(DefaultConfig())
	agent.mu.Lock()
	agent.state = AgentStateRunning
	agent.stopChan = make(chan struct{})
	agent.mu.Unlock()

	_, err := agent.Run(context.Background(), "hello")
	if err == nil {
		t.Error("expected error when agent is already running")
	}
}

func TestAgentCallback(t *testing.T) {
	var mu sync.Mutex
	var receivedEvents []Event

	cfg := DefaultConfig()
	cfg.Callback = func(evt Event) {
		mu.Lock()
		defer mu.Unlock()
		receivedEvents = append(receivedEvents, evt)
	}

	agent := NewAgent(cfg)
	if agent.callback == nil {
		t.Error("expected callback to be set")
	}
}

func TestExtractTextFromBlocks(t *testing.T) {
	blocks := []ContentBlock{
		TextBlock{Text: "hello "},
		ToolUseBlock{Name: "test", ID: "id1"},
		TextBlock{Text: "world"},
	}

	result := extractTextFromBlocks(blocks)
	if result != "hello world" {
		t.Errorf("expected 'hello world', got %q", result)
	}
}

func TestExtractToolUses(t *testing.T) {
	blocks := []ContentBlock{
		TextBlock{Text: "hello"},
		ToolUseBlock{Name: "tool1", ID: "id1"},
		ToolUseBlock{Name: "tool2", ID: "id2"},
	}

	uses := extractToolUses(blocks)
	if len(uses) != 2 {
		t.Fatalf("expected 2 tool uses, got %d", len(uses))
	}
	if uses[0].Name != "tool1" {
		t.Errorf("expected first tool to be 'tool1', got %s", uses[0].Name)
	}
	if uses[1].Name != "tool2" {
		t.Errorf("expected second tool to be 'tool2', got %s", uses[1].Name)
	}
}

func TestAgentEmit(t *testing.T) {
	agent := NewAgent(DefaultConfig())

	// Test with nil events channel (should not panic)
	agent.emit(nil, Event{Type: EventTurnStart, Data: map[string]any{"turn": 1}})

	// Test with valid channel
	events := make(chan Event, 10)
	agent.emit(events, Event{Type: EventTurnStart, Data: map[string]any{"turn": 1}})

	select {
	case evt := <-events:
		if evt.Type != EventTurnStart {
			t.Errorf("expected EventTurnStart, got %s", evt.Type)
		}
	default:
		t.Error("expected event to be sent to channel")
	}
}

func TestAgentCheckStop(t *testing.T) {
	agent := NewAgent(DefaultConfig())

	// Fresh agent should not be stopped
	if agent.checkStop() {
		t.Error("expected checkStop to return false for new agent")
	}

	// After stopping
	agent.Stop()
	if !agent.checkStop() {
		t.Error("expected checkStop to return true after Stop()")
	}
}

func TestAgentPauseResumeIntegration(t *testing.T) {
	agent := NewAgent(DefaultConfig())

	// Set to running
	agent.mu.Lock()
	agent.state = AgentStateRunning
	agent.stopChan = make(chan struct{})
	agent.mu.Unlock()

	// Pause
	agent.Pause()
	if agent.State() != AgentStatePaused {
		t.Fatalf("expected state paused, got %s", agent.State())
	}

	// Resume
	agent.Resume()
	if agent.State() != AgentStateRunning {
		t.Fatalf("expected state running, got %s", agent.State())
	}
}

func TestAgentMessageHistoryThreadSafety(t *testing.T) {
	agent := NewAgent(DefaultConfig())

	// Add some messages
	for i := 0; i < 10; i++ {
		agent.messageHistory = append(agent.messageHistory, UserMessage("msg"))
	}

	// Read from multiple goroutines
	var wg sync.WaitGroup
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			history := agent.MessageHistory()
			if len(history) != 10 {
				t.Errorf("expected 10 messages, got %d", len(history))
			}
		}()
	}
	wg.Wait()
}

func TestAgentMaxDuration(t *testing.T) {
	cfg := DefaultConfig()
	cfg.MaxDuration = 1 * time.Millisecond
	cfg.MaxTurns = 1

	agent := NewAgent(cfg)

	// This will fail because there's no API key, but we can test the config
	if agent.config.MaxDuration != 1*time.Millisecond {
		t.Errorf("expected MaxDuration to be 1ms, got %v", agent.config.MaxDuration)
	}
}

func TestAgentConcurrentToolExecution(t *testing.T) {
	agent := NewAgent(DefaultConfig())

	readOnlyTool := &mockTool{
		name:     "read_tool",
		readOnly: true,
		result:   Success("read result"),
	}
	writeTool := &mockTool{
		name:     "write_tool",
		readOnly: false,
		result:   Success("write result"),
	}

	agent.RegisterTool(readOnlyTool)
	agent.RegisterTool(writeTool)

	if !agent.GetTool("read_tool").IsReadOnly() {
		t.Error("expected read_tool to be read-only")
	}
	if agent.GetTool("write_tool").IsReadOnly() {
		t.Error("expected write_tool to not be read-only")
	}
}
