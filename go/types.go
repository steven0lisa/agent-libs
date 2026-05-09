package agentlib

import (
	"encoding/json"
	"fmt"
)

// Role represents the role of a message sender.
type Role string

const (
	RoleUser      Role = "user"
	RoleAssistant Role = "assistant"
)

// ContentBlockType represents the type of a content block.
type ContentBlockType string

const (
	BlockText       ContentBlockType = "text"
	BlockToolUse    ContentBlockType = "tool_use"
	BlockToolResult ContentBlockType = "tool_result"
	BlockThinking   ContentBlockType = "thinking"
)

// ContentBlock is the interface for all content block types.
type ContentBlock interface {
	BlockType() ContentBlockType
}

// TextBlock represents a text content block.
type TextBlock struct {
	Text string `json:"text"`
}

// BlockType returns the type of this block.
func (t TextBlock) BlockType() ContentBlockType { return BlockText }

// ToolUseBlock represents a tool use request from the assistant.
type ToolUseBlock struct {
	Name  string          `json:"name"`
	ID    string          `json:"id"`
	Input json.RawMessage `json:"input"`
}

// BlockType returns the type of this block.
func (t ToolUseBlock) BlockType() ContentBlockType { return BlockToolUse }

// ToolResultBlock represents the result of a tool execution.
type ToolResultBlock struct {
	ToolUseID   string    `json:"tool_use_id"`
	Content     string    `json:"content"`
	IsError     *bool     `json:"is_error,omitempty"`
	NewMessages []Message `json:"new_messages,omitempty"`
}

// BlockType returns the type of this block.
func (t ToolResultBlock) BlockType() ContentBlockType { return BlockToolResult }

// ThinkingBlock represents a thinking content block.
type ThinkingBlock struct {
	Thinking  string `json:"thinking"`
	Signature string `json:"signature,omitempty"`
}

// BlockType returns the type of this block.
func (t ThinkingBlock) BlockType() ContentBlockType { return BlockThinking }

// contentBlockWrapper is used for JSON marshaling/unmarshaling.
type contentBlockWrapper struct {
	Type      string          `json:"type"`
	Text      string          `json:"text,omitempty"`
	Name      string          `json:"name,omitempty"`
	ID        string          `json:"id,omitempty"`
	Input     json.RawMessage `json:"input,omitempty"`
	ToolUseID string          `json:"tool_use_id,omitempty"`
	Content   string          `json:"content,omitempty"`
	IsError   *bool           `json:"is_error,omitempty"`
	Thinking  string          `json:"thinking,omitempty"`
	Signature string          `json:"signature,omitempty"`
}

// MarshalJSON implements custom JSON marshaling for ContentBlock.
func (m Message) MarshalJSON() ([]byte, error) {
	type Alias Message
	return json.Marshal(&struct {
		*Alias
		Content []json.RawMessage `json:"content"`
	}{
		Alias:   (*Alias)(&m),
		Content: marshalContentBlocks(m.Content),
	})
}

// UnmarshalJSON implements custom JSON unmarshaling for Message.
func (m *Message) UnmarshalJSON(data []byte) error {
	type Alias Message
	aux := &struct {
		*Alias
		Content []contentBlockWrapper `json:"content"`
	}{
		Alias: (*Alias)(m),
	}
	if err := json.Unmarshal(data, aux); err != nil {
		return err
	}
	m.Content = unmarshalContentBlocks(aux.Content)
	return nil
}

func marshalContentBlocks(blocks []ContentBlock) []json.RawMessage {
	result := make([]json.RawMessage, len(blocks))
	for i, block := range blocks {
		result[i] = marshalContentBlock(block)
	}
	return result
}

func marshalContentBlock(block ContentBlock) json.RawMessage {
	switch b := block.(type) {
	case TextBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "text", Text: b.Text})
		return data
	case *TextBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "text", Text: b.Text})
		return data
	case ToolUseBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "tool_use", Name: b.Name, ID: b.ID, Input: b.Input})
		return data
	case *ToolUseBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "tool_use", Name: b.Name, ID: b.ID, Input: b.Input})
		return data
	case ToolResultBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "tool_result", ToolUseID: b.ToolUseID, Content: b.Content, IsError: b.IsError})
		return data
	case *ToolResultBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "tool_result", ToolUseID: b.ToolUseID, Content: b.Content, IsError: b.IsError})
		return data
	case ThinkingBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "thinking", Thinking: b.Thinking, Signature: b.Signature})
		return data
	case *ThinkingBlock:
		data, _ := json.Marshal(contentBlockWrapper{Type: "thinking", Thinking: b.Thinking, Signature: b.Signature})
		return data
	default:
		data, _ := json.Marshal(block)
		return data
	}
}

func unmarshalContentBlocks(wrappers []contentBlockWrapper) []ContentBlock {
	result := make([]ContentBlock, len(wrappers))
	for i, w := range wrappers {
		result[i] = unmarshalContentBlock(w)
	}
	return result
}

func unmarshalContentBlock(w contentBlockWrapper) ContentBlock {
	switch w.Type {
	case "text":
		return TextBlock{Text: w.Text}
	case "tool_use":
		return ToolUseBlock{Name: w.Name, ID: w.ID, Input: w.Input}
	case "tool_result":
		return ToolResultBlock{ToolUseID: w.ToolUseID, Content: w.Content, IsError: w.IsError}
	case "thinking":
		return ThinkingBlock{Thinking: w.Thinking, Signature: w.Signature}
	default:
		return TextBlock{Text: w.Content}
	}
}

// Message represents a message in the conversation.
type Message struct {
	Role    Role           `json:"role"`
	Content []ContentBlock `json:"content"`
}

// UserMessage creates a new user message with the given text.
func UserMessage(text string) Message {
	return Message{
		Role:    RoleUser,
		Content: []ContentBlock{TextBlock{Text: text}},
	}
}

// AssistantMessage creates a new assistant message with the given blocks.
func AssistantMessage(blocks []ContentBlock) Message {
	return Message{
		Role:    RoleAssistant,
		Content: blocks,
	}
}

// EventType represents the type of an event.
type EventType string

const (
	EventTurnStart     EventType = "turn_start"
	EventMessageStart  EventType = "message_start"
	EventMessageDelta  EventType = "message_delta"
	EventThinkingDelta EventType = "thinking_delta"
	EventMessageEnd    EventType = "message_end"
	EventToolUseStart  EventType = "tool_use_start"
	EventToolUseEnd    EventType = "tool_use_end"
	EventError         EventType = "error"
	EventComplete      EventType = "complete"
	EventCompact       EventType = "compact"
)

// Event represents an event in the agent's execution.
type Event struct {
	Type EventType      `json:"type"`
	Data map[string]any `json:"data"`
}

// String returns a string representation of the event.
func (e Event) String() string {
	return fmt.Sprintf("Event{type=%s, data=%v}", e.Type, e.Data)
}

// StreamEvent represents an event from the Anthropic API stream.
type StreamEvent struct {
	Type         string                 `json:"type"`
	Message      *APIResponse           `json:"message,omitempty"`
	Index        int                    `json:"index,omitempty"`
	ContentBlock map[string]any         `json:"content_block,omitempty"`
	Delta        map[string]any         `json:"delta,omitempty"`
	Usage        *Usage                 `json:"usage,omitempty"`
	Error        *APIError              `json:"error,omitempty"`
}

// APIError represents an error from the Anthropic API.
type APIError struct {
	Type    string `json:"type"`
	Message string `json:"message"`
}

// Error implements the error interface.
func (e *APIError) Error() string {
	return fmt.Sprintf("API error: %s", e.Message)
}
