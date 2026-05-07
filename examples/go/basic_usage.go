// Basic usage example: Run the agent with a simple prompt.
package main

import (
	"context"
	"fmt"
	"os"

	"github.com/steven0lisa/agent-libs/go"
)

func main() {
	// Configure the agent. API key comes from the caller's environment.
	cfg := agentlib.DefaultConfig()
	cfg.APIKey = os.Getenv("ANTHROPIC_AUTH_TOKEN")
	if cfg.APIKey == "" {
		cfg.APIKey = os.Getenv("ANTHROPIC_API_KEY")
	}
	if baseURL := os.Getenv("ANTHROPIC_BASE_URL"); baseURL != "" {
		cfg.BaseURL = baseURL
	}
	if model := os.Getenv("ANTHROPIC_MODEL"); model != "" {
		cfg.Model = model
	}

	agent := agentlib.NewAgent(cfg)
	ctx := context.Background()

	for event := range agent.Run(ctx, "Please list the files in the current directory") {
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
