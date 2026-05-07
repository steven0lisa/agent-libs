// Example: Enable subagent support and let the model delegate tasks.
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
	cfg.EnableSubagent = true
	cfg.SubagentMaxTurns = 20

	agent := agentlib.NewAgent(cfg)
	ctx := context.Background()

	for event := range agent.Run(ctx,
		"Explore this codebase using subagents to parallelize exploration") {
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
