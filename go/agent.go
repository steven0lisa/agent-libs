package agentlib

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/steven0lisa/agent-libs/go/skills"
)

// AgentState represents the lifecycle state of an Agent.
type AgentState string

const (
	AgentStateIdle      AgentState = "idle"
	AgentStateRunning   AgentState = "running"
	AgentStatePaused    AgentState = "paused"
	AgentStateStopping  AgentState = "stopping"
	AgentStateStopped   AgentState = "stopped"
	AgentStateCompleted AgentState = "completed"
	AgentStateError     AgentState = "error"
)

// Agent is the core agent implementation with lifecycle management.
type Agent struct {
	config         Config
	tools          map[string]Tool
	messageHistory []Message
	turnCount      int
	state          AgentState
	startTime      time.Time

	mu          sync.RWMutex
	pauseCond   *sync.Cond
	stopChan    chan struct{}
	callback    func(Event)

	skillLoader  *skills.Loader
	loadedSkills []skills.SkillInfo
}

// NewAgent creates a new Agent with the given configuration.
func NewAgent(cfg Config) *Agent {
	a := &Agent{
		config:         cfg,
		tools:          make(map[string]Tool),
		messageHistory: make([]Message, 0),
		turnCount:      0,
		state:          AgentStateIdle,
		stopChan:       make(chan struct{}),
		callback:       cfg.Callback,
	}
	a.pauseCond = sync.NewCond(&a.mu)

	if cfg.EnableSkills {
		a.initSkills()
	}

	return a
}

// RegisterTool registers a tool with the agent.
func (a *Agent) RegisterTool(tool Tool) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.tools[tool.Name()] = tool
}

// UnregisterTool removes a tool from the agent.
func (a *Agent) UnregisterTool(name string) {
	a.mu.Lock()
	defer a.mu.Unlock()
	delete(a.tools, name)
}

// ListTools returns a list of all registered tools.
func (a *Agent) ListTools() []Tool {
	a.mu.RLock()
	defer a.mu.RUnlock()
	tools := make([]Tool, 0, len(a.tools))
	for _, tool := range a.tools {
		tools = append(tools, tool)
	}
	return tools
}

// GetTool returns a tool by name, or nil if not found.
func (a *Agent) GetTool(name string) Tool {
	a.mu.RLock()
	defer a.mu.RUnlock()
	return a.tools[name]
}

// State returns the current state of the agent.
func (a *Agent) State() AgentState {
	a.mu.RLock()
	defer a.mu.RUnlock()
	return a.state
}

// setState sets the agent state (must hold lock).
func (a *Agent) setState(state AgentState) {
	a.state = state
}

// MessageHistory returns a copy of the message history.
func (a *Agent) MessageHistory() []Message {
	a.mu.RLock()
	defer a.mu.RUnlock()
	history := make([]Message, len(a.messageHistory))
	copy(history, a.messageHistory)
	return history
}

// ClearHistory clears the message history and resets the turn count.
func (a *Agent) ClearHistory() {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.messageHistory = a.messageHistory[:0]
	a.turnCount = 0
}

// TurnCount returns the current turn count.
func (a *Agent) TurnCount() int {
	a.mu.RLock()
	defer a.mu.RUnlock()
	return a.turnCount
}

// Pause pauses the agent. The current turn will finish, then the agent will pause.
func (a *Agent) Pause() {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.state == AgentStateRunning {
		a.state = AgentStatePaused
	}
}

// Resume resumes a paused agent.
func (a *Agent) Resume() {
	a.mu.Lock()
	if a.state == AgentStatePaused {
		a.state = AgentStateRunning
		a.pauseCond.Broadcast()
	}
	a.mu.Unlock()
}

// Stop stops the agent. Cannot be resumed.
func (a *Agent) Stop() {
	a.mu.Lock()
	if a.state == AgentStateRunning || a.state == AgentStatePaused || a.state == AgentStateIdle {
		a.state = AgentStateStopping
		close(a.stopChan)
		a.pauseCond.Broadcast()
	}
	a.mu.Unlock()
}

// waitIfPaused blocks until the agent is resumed or stopped.
// Must be called with lock held.
func (a *Agent) waitIfPaused() bool {
	for a.state == AgentStatePaused {
		a.pauseCond.Wait()
		if a.state == AgentStateStopping || a.state == AgentStateStopped {
			return false
		}
	}
	return a.state != AgentStateStopping && a.state != AgentStateStopped
}

// checkStop checks if the agent should stop.
func (a *Agent) checkStop() bool {
	select {
	case <-a.stopChan:
		return true
	default:
		return false
	}
}

// emit sends an event to the events channel and calls the callback if set.
func (a *Agent) emit(events chan<- Event, evt Event) {
	if events != nil {
		select {
		case events <- evt:
		case <-a.stopChan:
		}
	}
	if a.callback != nil {
		a.callback(evt)
	}
}

// initSkills initializes the skill loader and registers the skill tool.
func (a *Agent) initSkills() {
	opts := skills.LoaderOptions{
		SkillsDir:            a.config.SkillsDir,
		IncludeProjectSkills: a.config.IncludeProjectSkills,
		ProjectDir:           a.config.SkillsProjectDir,
	}
	a.skillLoader = skills.NewLoader(opts)
	a.RegisterTool(&skillToolAdapter{inner: skills.NewSkillTool(a.skillLoader)})
}

// skillToolAdapter wraps skills.SkillTool into an agentlib.Tool.
type skillToolAdapter struct {
	inner *skills.SkillTool
}

func (a *skillToolAdapter) Name() string              { return a.inner.ToolName() }
func (a *skillToolAdapter) Description() string       { return a.inner.ToolDescription() }
func (a *skillToolAdapter) InputSchema() map[string]any { return a.inner.ToolInputSchema() }
func (a *skillToolAdapter) IsReadOnly() bool          { return a.inner.ToolIsReadOnly() }
func (a *skillToolAdapter) Call(ctx context.Context, input map[string]any, toolCtx ToolContext) (ToolResult, error) {
	name := GetString(input, "skill", "")
	args := GetString(input, "args", "")
	result, err := a.inner.ExecuteWithArgs(name, args)
	if err != nil {
		return Error(err.Error()), nil
	}
	return Success(result), nil
}

// Run starts the agent with the given input.
// Returns a channel of events and an error if the agent cannot start.
func (a *Agent) Run(ctx context.Context, input string) (<-chan Event, error) {
	a.mu.Lock()
	defer a.mu.Unlock()

	if a.state == AgentStateRunning || a.state == AgentStatePaused {
		return nil, fmt.Errorf("agent is already running")
	}

	// Reset state
	a.state = AgentStateRunning
	a.stopChan = make(chan struct{})
	a.turnCount = 0
	a.startTime = time.Now()

	// Add user message
	a.messageHistory = append(a.messageHistory, UserMessage(input))

	events := make(chan Event, 100)
	go a.runLoop(ctx, events)

	return events, nil
}

// Chat continues the conversation with existing messages.
func (a *Agent) Chat(ctx context.Context, messages []Message) (<-chan Event, error) {
	a.mu.Lock()
	defer a.mu.Unlock()

	if a.state == AgentStateRunning || a.state == AgentStatePaused {
		return nil, fmt.Errorf("agent is already running")
	}

	// Reset state
	a.state = AgentStateRunning
	a.stopChan = make(chan struct{})
	a.turnCount = 0
	a.startTime = time.Now()

	// Add messages
	a.messageHistory = append(a.messageHistory, messages...)

	events := make(chan Event, 100)
	go a.runLoop(ctx, events)

	return events, nil
}

// runLoop is the main agent loop.
func (a *Agent) runLoop(ctx context.Context, events chan<- Event) {
	defer close(events)

	a.mu.RLock()
	tools := make(map[string]Tool, len(a.tools))
	for k, v := range a.tools {
		tools[k] = v
	}
	cfg := a.config
	a.mu.RUnlock()

	// Load skills if enabled
	if a.skillLoader != nil {
		a.loadedSkills = a.skillLoader.DiscoverAll()
	}

	systemPrompt := BuildSystemPrompt(tools, cfg.SystemPrompt, cfg.EnableSubagent, cfg.SubagentMaxTurns, a.loadedSkills)
	client := NewAPIClient(cfg)

	for {
		// Check stop
		if a.checkStop() {
			a.mu.Lock()
			a.state = AgentStateStopped
			a.mu.Unlock()
			return
		}

		// Check max turns
		a.mu.Lock()
		if a.turnCount >= cfg.MaxTurns {
			a.state = AgentStateError
			a.mu.Unlock()
			a.emit(events, Event{Type: EventError, Data: map[string]any{"message": "max turns reached"}})
			return
		}

		// Check duration limit
		if cfg.MaxDuration > 0 && time.Since(a.startTime) > cfg.MaxDuration {
			a.state = AgentStateError
			a.mu.Unlock()
			a.emit(events, Event{Type: EventError, Data: map[string]any{"message": "max duration exceeded"}})
			return
		}

		// Wait if paused
		if !a.waitIfPaused() {
			a.state = AgentStateStopped
			a.mu.Unlock()
			return
		}

		a.turnCount++
		turn := a.turnCount
		a.mu.Unlock()

		a.emit(events, Event{Type: EventTurnStart, Data: map[string]any{"turn": turn}})

		// Build tool definitions
		toolDefs := make([]ToolDefinition, 0, len(tools))
		for _, tool := range tools {
			toolDefs = append(toolDefs, ToToolDefinition(tool))
		}

		// Build messages for API
		a.mu.RLock()
		messages := make([]Message, len(a.messageHistory))
		copy(messages, a.messageHistory)
		a.mu.RUnlock()

		req := APIRequest{
			Model:     cfg.Model,
			Messages:  messages,
			System:    systemPrompt,
			Tools:     toolDefs,
			MaxTokens: cfg.MaxTokens,
			Stream:    cfg.Stream,
		}

		// Make API call
		var assistantContent []ContentBlock
		a.emit(events, Event{Type: EventMessageStart, Data: map[string]any{}})

		if cfg.Stream {
			stream, err := client.StreamMessages(ctx, req)
			if err != nil {
				a.emit(events, Event{Type: EventError, Data: map[string]any{"message": err.Error()}})
				a.mu.Lock()
				a.state = AgentStateError
				a.mu.Unlock()
				return
			}

			assistantContent = a.processStream(ctx, stream, events)
		} else {
			resp, err := client.SendMessages(ctx, req)
			if err != nil {
				a.emit(events, Event{Type: EventError, Data: map[string]any{"message": err.Error()}})
				a.mu.Lock()
				a.state = AgentStateError
				a.mu.Unlock()
				return
			}
			assistantContent = a.processResponse(resp, events)
		}

		a.emit(events, Event{Type: EventMessageEnd, Data: map[string]any{}})

		// Check stop after API call
		if a.checkStop() {
			a.mu.Lock()
			a.state = AgentStateStopped
			a.mu.Unlock()
			return
		}

		// Add assistant message to history
		a.mu.Lock()
		a.messageHistory = append(a.messageHistory, AssistantMessage(assistantContent))
		a.mu.Unlock()

		// Extract tool uses
		var toolUses []ToolUseBlock
		for _, block := range assistantContent {
			if tu, ok := block.(ToolUseBlock); ok {
				toolUses = append(toolUses, tu)
			}
		}

		if len(toolUses) == 0 {
			// Task complete
			finalText := extractTextFromBlocks(assistantContent)
			a.mu.Lock()
			a.state = AgentStateCompleted
			a.mu.Unlock()
			a.emit(events, Event{Type: EventComplete, Data: map[string]any{"final_content": finalText}})
			return
		}

		// Execute tools
		results := a.executeTools(ctx, toolUses, events, tools)

		// Check stop after tool execution
		if a.checkStop() {
			a.mu.Lock()
			a.state = AgentStateStopped
			a.mu.Unlock()
			return
		}

		// Add tool results to history
		resultBlocks := make([]ContentBlock, len(results))
		for i, r := range results {
			resultBlocks[i] = r
		}

		a.mu.Lock()
		a.messageHistory = append(a.messageHistory, Message{
			Role:    RoleUser,
			Content: resultBlocks,
		})
		a.mu.Unlock()
	}
}

// processStream processes a stream of events from the API.
func (a *Agent) processStream(ctx context.Context, stream <-chan StreamEvent, events chan<- Event) []ContentBlock {
	var content []ContentBlock
	var currentText strings.Builder
	var currentThinking strings.Builder

	for {
		select {
		case event, ok := <-stream:
			if !ok {
				// Stream closed
				if currentText.Len() > 0 {
					content = append(content, TextBlock{Text: currentText.String()})
				}
				if currentThinking.Len() > 0 {
					content = append(content, ThinkingBlock{Thinking: currentThinking.String()})
				}
				return content
			}

			switch event.Type {
			case "content_block_start":
				if event.ContentBlock != nil {
					block := ParseContentBlock(event.ContentBlock)
					content = append(content, block)
				}

			case "content_block_delta":
				if event.Delta != nil {
					deltaType, _ := event.Delta["type"].(string)
					switch deltaType {
					case "text_delta":
						text, _ := event.Delta["text"].(string)
						currentText.WriteString(text)
						a.emit(events, Event{Type: EventMessageDelta, Data: map[string]any{"text": text}})
					case "thinking_delta":
						thinking, _ := event.Delta["thinking"].(string)
						currentThinking.WriteString(thinking)
						a.emit(events, Event{Type: EventThinkingDelta, Data: map[string]any{"thinking": thinking}})
					}
				}

			case "message_stop":
				if currentText.Len() > 0 {
					content = append(content, TextBlock{Text: currentText.String()})
				}
				if currentThinking.Len() > 0 {
					content = append(content, ThinkingBlock{Thinking: currentThinking.String()})
				}
				return content
			}

		case <-ctx.Done():
			return content
		case <-a.stopChan:
			return content
		}
	}
}

// processResponse processes a non-streaming API response.
func (a *Agent) processResponse(resp *APIResponse, events chan<- Event) []ContentBlock {
	var content []ContentBlock
	for _, blockData := range resp.Content {
		block := ParseContentBlock(blockData)
		content = append(content, block)
		if tb, ok := block.(TextBlock); ok {
			a.emit(events, Event{Type: EventMessageDelta, Data: map[string]any{"text": tb.Text}})
		}
	}
	return content
}

// executeTools executes the given tool uses and returns the results.
func (a *Agent) executeTools(ctx context.Context, toolUses []ToolUseBlock, events chan<- Event, tools map[string]Tool) []ToolResultBlock {
	// Group by read-only
	var readOnly []ToolUseBlock
	var write []ToolUseBlock

	for _, tu := range toolUses {
		if tool, ok := tools[tu.Name]; ok && tool.IsReadOnly() {
			readOnly = append(readOnly, tu)
		} else {
			write = append(write, tu)
		}
	}

	results := make([]ToolResultBlock, 0, len(toolUses))
	var mu sync.Mutex
	var wg sync.WaitGroup

	// Concurrent execution of read-only tools
	for _, tu := range readOnly {
		wg.Add(1)
		go func(tu ToolUseBlock) {
			defer wg.Done()
			result := a.executeSingleTool(ctx, tu, events, tools)
			mu.Lock()
			results = append(results, result)
			mu.Unlock()
		}(tu)
	}
	wg.Wait()

	// Sequential execution of write tools
	for _, tu := range write {
		if a.checkStop() {
			break
		}
		result := a.executeSingleTool(ctx, tu, events, tools)
		results = append(results, result)
	}

	return results
}

// executeSingleTool executes a single tool use and returns the result.
func (a *Agent) executeSingleTool(ctx context.Context, tu ToolUseBlock, events chan<- Event, tools map[string]Tool) ToolResultBlock {
	a.emit(events, Event{Type: EventToolUseStart, Data: map[string]any{
		"name": tu.Name, "id": tu.ID,
	}})

	tool, ok := tools[tu.Name]
	if !ok {
		result := ToolResultBlock{
			ToolUseID: tu.ID,
			Content:   fmt.Sprintf("Tool not found: %s", tu.Name),
			IsError:   boolPtr(true),
		}
		a.emit(events, Event{Type: EventToolUseEnd, Data: map[string]any{
			"name": tu.Name, "id": tu.ID, "result": result,
		}})
		return result
	}

	var input map[string]any
	if err := json.Unmarshal(tu.Input, &input); err != nil {
		result := ToolResultBlock{
			ToolUseID: tu.ID,
			Content:   fmt.Sprintf("Failed to parse tool input: %v", err),
			IsError:   boolPtr(true),
		}
		a.emit(events, Event{Type: EventToolUseEnd, Data: map[string]any{
			"name": tu.Name, "id": tu.ID, "result": result,
		}})
		return result
	}

	a.mu.RLock()
	toolCtx := ToolContext{
		WorkDir:        a.config.WorkDir,
		MessageHistory: a.messageHistory,
	}
	a.mu.RUnlock()

	result, err := tool.Call(ctx, input, toolCtx)
	if err != nil {
		result = ToolResult{Content: err.Error(), IsError: true}
	}

	resultBlock := ToolResultBlock{
		ToolUseID: tu.ID,
		Content:   result.Content,
		IsError:   boolPtr(result.IsError),
	}

	a.emit(events, Event{Type: EventToolUseEnd, Data: map[string]any{
		"name": tu.Name, "id": tu.ID, "result": result,
	}})

	return resultBlock
}

// extractTextFromBlocks extracts all text from content blocks.
func extractTextFromBlocks(blocks []ContentBlock) string {
	var sb strings.Builder
	for _, block := range blocks {
		switch b := block.(type) {
		case TextBlock:
			sb.WriteString(b.Text)
		case *TextBlock:
			sb.WriteString(b.Text)
		}
	}
	return sb.String()
}

// extractToolUses extracts all ToolUseBlocks from content blocks.
func extractToolUses(blocks []ContentBlock) []ToolUseBlock {
	var uses []ToolUseBlock
	for _, block := range blocks {
		if tu, ok := block.(ToolUseBlock); ok {
			uses = append(uses, tu)
		}
	}
	return uses
}
