// Example: Register a custom tool with the agent.
package main

import (
	"context"
	"fmt"
	"os"
	"strconv"

	"github.com/steven0lisa/agent-libs/go"
)

// CalculatorTool performs basic arithmetic.
type CalculatorTool struct{}

func (c *CalculatorTool) Name() string        { return "calculator" }
func (c *CalculatorTool) Description() string { return "Perform basic arithmetic." }
func (c *CalculatorTool) IsReadOnly() bool    { return true }

func (c *CalculatorTool) InputSchema() map[string]any {
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			"operation": map[string]any{"type": "string", "enum": []string{"add", "subtract", "multiply", "divide"}},
			"a":         map[string]any{"type": "number"},
			"b":         map[string]any{"type": "number"},
		},
		"required": []string{"operation", "a", "b"},
	}
}

func (c *CalculatorTool) Call(_ context.Context, input map[string]any, _ agentlib.ToolContext) (agentlib.ToolResult, error) {
	op := input["operation"].(string)
	a, _ := strconv.ParseFloat(fmt.Sprintf("%v", input["a"]), 64)
	b, _ := strconv.ParseFloat(fmt.Sprintf("%v", input["b"]), 64)

	var result float64
	switch op {
	case "add":
		result = a + b
	case "subtract":
		result = a - b
	case "multiply":
		result = a * b
	case "divide":
		if b == 0 {
			return agentlib.ToolResult{Content: "Cannot divide by zero", IsError: true}, nil
		}
		result = a / b
	default:
		return agentlib.ToolResult{Content: "Unknown operation: " + op, IsError: true}, nil
	}

	return agentlib.ToolResult{Content: fmt.Sprintf("%g", result), IsError: false}, nil
}

func main() {
	cfg := agentlib.DefaultConfig()
	cfg.APIKey = os.Getenv("ANTHROPIC_AUTH_TOKEN")
	agent := agentlib.NewAgent(cfg)
	agent.RegisterTool(&CalculatorTool{})

	ctx := context.Background()
	for event := range agent.Run(ctx, "What is 123 multiplied by 456?") {
		switch event.Type {
		case agentlib.EventMessageDelta:
			fmt.Print(event.Data["text"])
		case agentlib.EventComplete:
			fmt.Printf("\n\n[Complete] %s\n", event.Data["final_content"])
		case agentlib.EventError:
			fmt.Printf("\n[Error] %s\n", event.Data["message"])
		}
	}
}
