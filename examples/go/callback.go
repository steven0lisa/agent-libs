// Example: Use a callback to log every step of the agent's execution.
package main

import (
	"context"
	"fmt"
	"os"

	"github.com/steven0lisa/agent-libs/go"
)

func main() {
	cfg := agentlib.DefaultConfig()
	cfg.APIKey = os.Getenv("ANTHROPIC_AUTH_TOKEN")

	// Set a callback to log every event.
	cfg.Callback = func(event agentlib.Event) {
		switch event.Type {
		case agentlib.EventToolUseStart:
			fmt.Printf("[Callback] Tool '%s' called\n", event.Data["name"])
		case agentlib.EventToolUseEnd:
			result := event.Data["result"].(agentlib.ToolResult)
			status := "OK"
			if result.IsError {
				status = "ERROR"
			}
			fmt.Printf("[Callback] Tool '%s' finished: %s\n", event.Data["name"], status)
		case agentlib.EventError:
			fmt.Printf("[Callback] Error: %s\n", event.Data["message"])
		}
	}

	agent := agentlib.NewAgent(cfg)
	ctx := context.Background()

	for event := range agent.Run(ctx, "Read README.md and summarize it") {
		switch event.Type {
		case agentlib.EventMessageDelta:
			fmt.Print(event.Data["text"])
		case agentlib.EventComplete:
			fmt.Printf("\n\n[Complete] %s\n", event.Data["final_content"])
		}
	}
}
